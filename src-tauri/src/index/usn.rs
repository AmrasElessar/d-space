// SPDX-License-Identifier: GPL-3.0-or-later
//
// USN_RECORD_V2 parser + FSCTL bağlantıları — Sprint 3.8.
//
// Discovery Log #005 — Bölüm 5.6: USN Journal modeli "Everything"in temel
// mekanizmasıdır. NTFS USN_RECORD_V2 binary formatı:
//
//   DWORD  RecordLength;
//   WORD   MajorVersion;       // 2
//   WORD   MinorVersion;       // 0
//   DWORDLONG FileReferenceNumber;
//   DWORDLONG ParentFileReferenceNumber;
//   USN     Usn;               // i64
//   LARGE_INTEGER TimeStamp;   // FILETIME 100ns since 1601
//   DWORD   Reason;
//   DWORD   SourceInfo;
//   DWORD   SecurityId;
//   DWORD   FileAttributes;
//   WORD    FileNameLength;    // bytes (NOT chars)
//   WORD    FileNameOffset;
//   WCHAR   FileName[1];       // UTF-16 LE, NOT null-terminated
//
// NTFS file reference number 64-bit, alt 48 bit segment numarası (MFT
// record index), üst 16 bit sequence numarası. Persistent index için
// yalnız segment numarasını saklıyoruz (rename sırasında sequence değişir
// ama segment kalır — kayıt sürekliliği için tercih edilir).
//
// FSCTL constants `windows-rs` 0.61'in `Win32::System::Ioctl`'ünde:
//   * FSCTL_ENUM_USN_DATA       = 0x000900B3 (baseline enumerate)
//   * FSCTL_READ_USN_JOURNAL    = 0x000900BB (delta read)
//   * FSCTL_QUERY_USN_JOURNAL   = 0x000900F4 (journal_id okumak için)
// Windows 11'de bu sembolik isimler `windows` crate'inde değil; hand-roll
// edilir (FSCTL kodları kararlı, Win2K'dan beri değişmedi).
//
// USN reason bayrakları (`winioctl.h`):
//   USN_REASON_FILE_CREATE     = 0x00000100
//   USN_REASON_FILE_DELETE     = 0x00000200
//   USN_REASON_DATA_OVERWRITE  = 0x00000001
//   USN_REASON_RENAME_NEW_NAME = 0x00002000
//   USN_REASON_RENAME_OLD_NAME = 0x00001000

use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};

/// USN reason mask — delta uygularken yalnız bu olaylara tepki veriyoruz
/// (data değişim, oluşturma, silme, ad değişimi). Discovery #005 — diğer
/// reason'lar (REASON_SECURITY_CHANGE, REASON_BASIC_INFO_CHANGE vb.)
/// dosya isim/varlık bilgisini değiştirmez, görmezden gelinir.
pub const USN_REASON_MASK: u32 = USN_REASON_FILE_CREATE
    | USN_REASON_FILE_DELETE
    | USN_REASON_DATA_OVERWRITE
    | USN_REASON_RENAME_NEW_NAME
    | USN_REASON_RENAME_OLD_NAME
    | USN_REASON_CLOSE;

pub const USN_REASON_DATA_OVERWRITE: u32 = 0x0000_0001;
pub const USN_REASON_FILE_CREATE: u32 = 0x0000_0100;
pub const USN_REASON_FILE_DELETE: u32 = 0x0000_0200;
pub const USN_REASON_RENAME_OLD_NAME: u32 = 0x0000_1000;
pub const USN_REASON_RENAME_NEW_NAME: u32 = 0x0000_2000;
pub const USN_REASON_CLOSE: u32 = 0x8000_0000;

/// FSCTL_ENUM_USN_DATA — baseline enumerate.
pub const FSCTL_ENUM_USN_DATA: u32 = 0x0009_00B3;
/// FSCTL_READ_USN_JOURNAL — delta okuma.
pub const FSCTL_READ_USN_JOURNAL: u32 = 0x0009_00BB;
/// FSCTL_QUERY_USN_JOURNAL — journal_id sorgu.
pub const FSCTL_QUERY_USN_JOURNAL: u32 = 0x0009_00F4;

/// USN_RECORD_V2 sabit alan boyutu (FileName hariç) = 60 bayt.
pub const USN_RECORD_V2_HEADER_SIZE: usize = 60;

/// FILE_ATTRIBUTE_DIRECTORY (winnt.h).
pub const FILE_ATTRIBUTE_DIRECTORY: u32 = 0x0000_0010;

/// Win32 ERROR_JOURNAL_NOT_ACTIVE — USN journal kapalı volume için
/// `DeviceIoControl` bu hatayı verir. Caller wraparound veya re-create
/// kararı verir (D-Space: watcher başlatılmaz).
pub const ERROR_JOURNAL_NOT_ACTIVE: u32 = 1_179;
/// Win32 ERROR_JOURNAL_DELETE_IN_PROGRESS.
pub const ERROR_JOURNAL_DELETE_IN_PROGRESS: u32 = 1_178;
/// Win32 ERROR_JOURNAL_ENTRY_DELETED — istenen USN penceresi düşmüş
/// (truncation), wraparound → full re-enumerate.
pub const ERROR_JOURNAL_ENTRY_DELETED: u32 = 1_181;

/// USN_RECORD_V2 binary kaydının parse edilmiş hali. Sadece bizi
/// ilgilendiren alanlar tutuluyor — TimeStamp/SecurityId atılıyor.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UsnRecord {
    pub file_ref: i64,
    pub parent_ref: i64,
    pub usn_id: i64,
    pub reason: u32,
    pub attributes: u32,
    pub name: String,
}

impl UsnRecord {
    pub fn is_directory(&self) -> bool {
        self.attributes & FILE_ATTRIBUTE_DIRECTORY != 0
    }
}

/// NTFS file reference number → persistent segment kısmı (alt 48 bit).
/// Üst 16 bit sequence number; rename'de değişir, persist için tercih
/// edilmez.
#[inline]
pub fn segment_of(file_ref: u64) -> i64 {
    (file_ref & 0x0000_FFFF_FFFF_FFFF) as i64
}

/// Tek bir USN_RECORD_V2'yi binary'den parse eder. `buf` kayıt başlangıcı.
/// Dönen değer: (kayıt, kayıt boyutu — caller advance için).
pub fn parse_record_v2(buf: &[u8]) -> Result<(UsnRecord, usize)> {
    if buf.len() < USN_RECORD_V2_HEADER_SIZE {
        return Err(Error::Index(format!(
            "USN_RECORD_V2 buffer çok kısa: {} < {}",
            buf.len(),
            USN_RECORD_V2_HEADER_SIZE
        )));
    }
    let record_length = u32::from_le_bytes(buf[0..4].try_into().unwrap()) as usize;
    let major = u16::from_le_bytes(buf[4..6].try_into().unwrap());
    let _minor = u16::from_le_bytes(buf[6..8].try_into().unwrap());
    if major != 2 {
        return Err(Error::Index(format!(
            "Yalnız USN_RECORD_V2 destekleniyor, gelen MajorVersion={}",
            major
        )));
    }
    if record_length < USN_RECORD_V2_HEADER_SIZE || record_length > buf.len() {
        return Err(Error::Index(format!(
            "USN_RECORD_V2 record_length geçersiz: {} (buf={})",
            record_length,
            buf.len()
        )));
    }
    let file_ref_raw = u64::from_le_bytes(buf[8..16].try_into().unwrap());
    let parent_ref_raw = u64::from_le_bytes(buf[16..24].try_into().unwrap());
    let usn_id = i64::from_le_bytes(buf[24..32].try_into().unwrap());
    // 32..40 TimeStamp — atlanıyor.
    let reason = u32::from_le_bytes(buf[40..44].try_into().unwrap());
    // 44..48 SourceInfo, 48..52 SecurityId — atlanıyor.
    let attributes = u32::from_le_bytes(buf[52..56].try_into().unwrap());
    let name_length = u16::from_le_bytes(buf[56..58].try_into().unwrap()) as usize;
    let name_offset = u16::from_le_bytes(buf[58..60].try_into().unwrap()) as usize;

    if name_offset + name_length > record_length {
        return Err(Error::Index(format!(
            "USN_RECORD_V2 isim aralığı kayıt dışı: offset={} len={} record={}",
            name_offset, name_length, record_length
        )));
    }
    if name_length % 2 != 0 {
        return Err(Error::Index(format!(
            "USN_RECORD_V2 isim uzunluğu tek (UTF-16 LE 2-bayt): {}",
            name_length
        )));
    }
    let name_bytes = &buf[name_offset..name_offset + name_length];
    let utf16: Vec<u16> = name_bytes
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    let name = String::from_utf16(&utf16)
        .map_err(|e| Error::Index(format!("UTF-16 decode hatası: {}", e)))?;

    Ok((
        UsnRecord {
            file_ref: segment_of(file_ref_raw),
            parent_ref: segment_of(parent_ref_raw),
            usn_id,
            reason,
            attributes,
            name,
        },
        record_length,
    ))
}

/// Bir buffer içindeki tüm USN_RECORD_V2 kayıtlarını dizi olarak çıkar.
/// `offset`: ilk record başlangıcı (FSCTL output buffer'da ilk 8 bayt
/// `NextUsn` üzerinden gelir; caller doğru offset'i geçer).
pub fn parse_records(buf: &[u8], mut offset: usize) -> Result<Vec<UsnRecord>> {
    let mut out = Vec::new();
    while offset + USN_RECORD_V2_HEADER_SIZE <= buf.len() {
        let (record, len) = parse_record_v2(&buf[offset..])?;
        if len == 0 {
            break;
        }
        out.push(record);
        offset += len;
    }
    Ok(out)
}

/// Test fixture'larında V2 record bayt buffer'ı kurmak için yardımcı.
#[cfg(test)]
pub(crate) fn build_v2_record(
    file_ref: u64,
    parent_ref: u64,
    usn_id: i64,
    reason: u32,
    attributes: u32,
    name: &str,
) -> Vec<u8> {
    let utf16: Vec<u16> = name.encode_utf16().collect();
    let name_bytes: Vec<u8> = utf16.iter().flat_map(|c| c.to_le_bytes()).collect();
    let name_offset = USN_RECORD_V2_HEADER_SIZE;
    let name_len_bytes = name_bytes.len();
    // 8-bayt hizalanmış kayıt — Windows USN dökümünden: kayıt uzunluğu
    // 8-bayt katlarına yuvarlanır.
    let unaligned = USN_RECORD_V2_HEADER_SIZE + name_len_bytes;
    let record_length = unaligned.div_ceil(8) * 8;
    let mut buf = vec![0u8; record_length];
    buf[0..4].copy_from_slice(&(record_length as u32).to_le_bytes());
    buf[4..6].copy_from_slice(&2u16.to_le_bytes());
    buf[6..8].copy_from_slice(&0u16.to_le_bytes());
    buf[8..16].copy_from_slice(&file_ref.to_le_bytes());
    buf[16..24].copy_from_slice(&parent_ref.to_le_bytes());
    buf[24..32].copy_from_slice(&usn_id.to_le_bytes());
    // 32..40 timestamp = 0
    buf[40..44].copy_from_slice(&reason.to_le_bytes());
    // 44..48 sourceinfo, 48..52 securityid = 0
    buf[52..56].copy_from_slice(&attributes.to_le_bytes());
    buf[56..58].copy_from_slice(&(name_len_bytes as u16).to_le_bytes());
    buf[58..60].copy_from_slice(&(name_offset as u16).to_le_bytes());
    buf[name_offset..name_offset + name_len_bytes].copy_from_slice(&name_bytes);
    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_record_v2_roundtrip() {
        // Klasik bir dosya oluşturma kaydı: report.docx, parent 5
        let buf = build_v2_record(
            0x0000_0000_0001_2345_u64, // file_ref segment 0x12345
            0x0000_0000_0000_0005_u64, // parent_ref segment 5
            42,
            USN_REASON_FILE_CREATE | USN_REASON_CLOSE,
            0,
            "report.docx",
        );
        let (rec, sz) = parse_record_v2(&buf).expect("parse");
        assert_eq!(rec.file_ref, 0x12345);
        assert_eq!(rec.parent_ref, 5);
        assert_eq!(rec.usn_id, 42);
        assert_eq!(rec.name, "report.docx");
        assert_eq!(rec.reason & USN_REASON_FILE_CREATE, USN_REASON_FILE_CREATE);
        assert!(!rec.is_directory());
        // Kayıt boyutu 8-bayt hizalanmış olmalı
        assert!(sz % 8 == 0);
        assert!(sz >= USN_RECORD_V2_HEADER_SIZE + "report.docx".len() * 2);
    }

    #[test]
    fn parse_record_v2_utf16_unicode_name() {
        let buf = build_v2_record(7, 5, 1, USN_REASON_FILE_CREATE, 0, "rapor_çğüöş.txt");
        let (rec, _) = parse_record_v2(&buf).unwrap();
        assert_eq!(rec.name, "rapor_çğüöş.txt");
    }

    #[test]
    fn parse_record_v2_directory_attribute() {
        let buf = build_v2_record(
            10,
            5,
            1,
            USN_REASON_FILE_CREATE,
            FILE_ATTRIBUTE_DIRECTORY,
            "Documents",
        );
        let (rec, _) = parse_record_v2(&buf).unwrap();
        assert!(rec.is_directory());
        assert_eq!(
            rec.attributes & FILE_ATTRIBUTE_DIRECTORY,
            FILE_ATTRIBUTE_DIRECTORY
        );
    }

    #[test]
    fn parse_record_v2_short_buffer_errors() {
        let short = vec![0u8; 10];
        let err = parse_record_v2(&short).unwrap_err();
        assert!(matches!(err, Error::Index(_)));
    }

    #[test]
    fn parse_record_v2_wrong_major_version_errors() {
        let mut buf = build_v2_record(1, 2, 3, 0, 0, "x.txt");
        buf[4..6].copy_from_slice(&3u16.to_le_bytes()); // major=3
        let err = parse_record_v2(&buf).unwrap_err();
        match err {
            Error::Index(m) => assert!(m.contains("MajorVersion")),
            _ => panic!("Index variant beklendi"),
        }
    }

    #[test]
    fn parse_records_handles_multiple() {
        let mut buf = Vec::new();
        buf.extend_from_slice(&build_v2_record(
            1,
            5,
            10,
            USN_REASON_FILE_CREATE,
            0,
            "a.txt",
        ));
        buf.extend_from_slice(&build_v2_record(
            2,
            5,
            11,
            USN_REASON_FILE_CREATE,
            0,
            "b.txt",
        ));
        buf.extend_from_slice(&build_v2_record(
            3,
            5,
            12,
            USN_REASON_FILE_CREATE,
            FILE_ATTRIBUTE_DIRECTORY,
            "subdir",
        ));
        let recs = parse_records(&buf, 0).unwrap();
        assert_eq!(recs.len(), 3);
        assert_eq!(recs[0].name, "a.txt");
        assert_eq!(recs[1].name, "b.txt");
        assert!(recs[2].is_directory());
    }

    #[test]
    fn segment_strips_sequence_number() {
        // Üst 16 bit sequence, alt 48 bit segment
        let raw: u64 = 0x0012_3456_789A_BCDE;
        let seg = segment_of(raw);
        assert_eq!(seg, 0x0000_3456_789A_BCDE_i64);
    }

    #[test]
    fn reason_mask_includes_expected_events() {
        // Compile-time'da bit'lerin maskeye dahil olduğunu doğrula —
        // constant-fold edilen assertion'ları clippy const block ister.
        const _CHECK_CREATE: () = assert!(USN_REASON_MASK & USN_REASON_FILE_CREATE != 0);
        const _CHECK_DELETE: () = assert!(USN_REASON_MASK & USN_REASON_FILE_DELETE != 0);
        const _CHECK_RENAME_NEW: () = assert!(USN_REASON_MASK & USN_REASON_RENAME_NEW_NAME != 0);
        const _CHECK_RENAME_OLD: () = assert!(USN_REASON_MASK & USN_REASON_RENAME_OLD_NAME != 0);
        const _CHECK_OVERWRITE: () = assert!(USN_REASON_MASK & USN_REASON_DATA_OVERWRITE != 0);
    }
}
