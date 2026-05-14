// SPDX-License-Identifier: GPL-3.0-or-later
//
// MFT full walker — Bölüm 5.1, Bölüm 4.3 (Adım 2: Rust MFT'yi sıralı okur).
//
// Bu sprint hedefi: tüm MFT record'larını sırayla gezip in-use / dir / file
// sayımı ve örnek dosya adları çıkarma. Hiyerarşi kurma ve `Arc<ScanTree>`
// build sonraki sprint (Bölüm 4.3 Adım 3 + rayon paralel aggregation).
//
// Performans notu (Bölüm 5.4): 1TB SSD < 5sn hedefine ulaşmak için
// gerçek production'da bulk MFT read (raw IOCTL) ve rayon paralel parse
// gerekecek. Bu v0.1 walker tek thread ve `ntfs::Ntfs::file()` per-record
// çağırır — yeterli, sonraki turda optimize.

use crate::error::{Error, Result};
use ntfs::structured_values::NtfsFileNamespace;
use ntfs::{KnownNtfsFileRecordNumber, Ntfs, NtfsFile, NtfsFileFlags};
use serde::Serialize;
use std::fs::File;
use std::time::Instant;
use tracing::{debug, info};

const SAMPLE_NAME_LIMIT: usize = 25;
const MAX_RECORD_HARD_LIMIT: u64 = 50_000_000; // güvenlik tavanı

#[derive(Debug, Clone, Serialize)]
pub struct MftWalkStats {
    pub drive: String,
    pub volume_path: String,
    pub total_records_estimate: u64,
    pub records_visited: u64,
    pub in_use_records: u64,
    pub directory_records: u64,
    pub file_records: u64,
    pub skipped_errors: u64,
    pub bytes_aggregate: u64,
    pub sample_names: Vec<String>,
    pub elapsed_ms: u64,
}

/// "C", "c:", "C:\Users" → r"\\.\C:"
fn normalize_volume_path(drive: &str) -> Result<String> {
    let letter = drive
        .chars()
        .find(|c| c.is_ascii_alphabetic())
        .ok_or_else(|| Error::Scan(format!("Geçersiz sürücü: '{}'", drive)))?
        .to_ascii_uppercase();
    Ok(format!(r"\\.\{}:", letter))
}

/// MFT'nin kendi `$DATA` boyutundan toplam record sayısını tahmin eder.
fn estimate_record_count(ntfs: &Ntfs, handle: &mut File) -> Result<u64> {
    let mft_file = ntfs
        .file(handle, KnownNtfsFileRecordNumber::MFT as u64)
        .map_err(|e| Error::Scan(format!("MFT record 0 okunamadı: {:?}", e)))?;

    let data_attr_item = mft_file
        .data(handle, "")
        .ok_or_else(|| Error::Scan("MFT $DATA bulunamadı".into()))?
        .map_err(|e| Error::Scan(format!("MFT $DATA hatası: {:?}", e)))?;

    let attr = data_attr_item
        .to_attribute()
        .map_err(|e| Error::Scan(format!("MFT $DATA attribute: {:?}", e)))?;

    let total_bytes = attr.value_length();
    let record_size = ntfs.file_record_size() as u64;
    if record_size == 0 {
        return Err(Error::Scan("file_record_size = 0".into()));
    }
    Ok(total_bytes / record_size)
}

/// Tek bir `NtfsFile` için isim çıkarmaya çalışır. Win32 namespace tercih
/// edilir (DOS 8.3'ün ikincil görünümlerini atlamak için).
fn extract_name(file: &NtfsFile, handle: &mut File) -> Option<String> {
    // Önce Win32 namespace
    if let Some(Ok(name)) = file.name(handle, Some(NtfsFileNamespace::Win32), None) {
        return Some(name.name().to_string_lossy());
    }
    // Fallback: Win32AndDos (3) — birleşik isim
    if let Some(Ok(name)) = file.name(handle, None, None) {
        // DOS'u atla
        if !matches!(name.namespace(), NtfsFileNamespace::Dos) {
            return Some(name.name().to_string_lossy());
        }
    }
    None
}

/// Tüm MFT record'larını sırayla gezer. İlk 16 system file (record 0-15)
/// hariç tutulur — onlar `$MFT`, `$LogFile`, `$Volume`, `$AttrDef`, `$.`,
/// `$Bitmap`, `$Boot`, `$BadClus`, `$Secure`, `$UpCase`, `$Extend`, vb.
pub fn walk_mft(drive: &str) -> Result<MftWalkStats> {
    let start = Instant::now();
    let volume_path = normalize_volume_path(drive)?;

    debug!(volume = %volume_path, "MFT full walk başlıyor");
    let mut handle = File::open(&volume_path).map_err(|e| {
        Error::Scan(format!(
            "Volume açılamadı '{}': {} (yönetici izni gerekli olabilir)",
            volume_path, e
        ))
    })?;

    let ntfs = Ntfs::new(&mut handle)
        .map_err(|e| Error::Scan(format!("NTFS parse hatası: {:?}", e)))?;

    let estimated = estimate_record_count(&ntfs, &mut handle)?;
    let cap = estimated.min(MAX_RECORD_HARD_LIMIT);
    info!(estimated, cap, "MFT record sayısı tahmini");

    let mut visited = 0u64;
    let mut in_use = 0u64;
    let mut dirs = 0u64;
    let mut files = 0u64;
    let mut skipped = 0u64;
    let mut total_bytes = 0u64;
    let mut samples: Vec<String> = Vec::with_capacity(SAMPLE_NAME_LIMIT);

    // System records (0-15) atlanır — gerçek kullanıcı içeriği 16'dan başlar.
    for record_no in 16..cap {
        visited += 1;

        let file = match ntfs.file(&mut handle, record_no) {
            Ok(f) => f,
            Err(_) => {
                skipped += 1;
                continue;
            }
        };

        let flags = file.flags();
        if !flags.contains(NtfsFileFlags::IN_USE) {
            continue;
        }
        in_use += 1;

        let is_dir = file.is_directory();
        if is_dir {
            dirs += 1;
        } else {
            files += 1;
            // Resident veya non-resident $DATA boyutu (Bölüm 7 uyumlu boyut).
            // `data_size()` u32 — büyük dosyalar için NtfsFileName.data_size()
            // 64-bit verir; ileride per-file metadata'da o kullanılır.
            total_bytes = total_bytes.saturating_add(file.data_size() as u64);
        }

        if samples.len() < SAMPLE_NAME_LIMIT {
            if let Some(name) = extract_name(&file, &mut handle) {
                samples.push(name);
            }
        }
    }

    let stats = MftWalkStats {
        drive: drive.to_string(),
        volume_path,
        total_records_estimate: estimated,
        records_visited: visited,
        in_use_records: in_use,
        directory_records: dirs,
        file_records: files,
        skipped_errors: skipped,
        bytes_aggregate: total_bytes,
        sample_names: samples,
        elapsed_ms: start.elapsed().as_millis() as u64,
    };

    info!(
        visited = stats.records_visited,
        in_use = stats.in_use_records,
        dirs = stats.directory_records,
        files = stats.file_records,
        gb = stats.bytes_aggregate / 1_073_741_824,
        skipped = stats.skipped_errors,
        elapsed_ms = stats.elapsed_ms,
        "MFT walk tamamlandı"
    );

    Ok(stats)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_drive_paths() {
        assert_eq!(normalize_volume_path("E").unwrap(), r"\\.\E:");
        assert_eq!(normalize_volume_path("d:").unwrap(), r"\\.\D:");
        assert!(normalize_volume_path("").is_err());
    }
}
