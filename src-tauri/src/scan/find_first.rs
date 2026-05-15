// SPDX-License-Identifier: GPL-3.0-or-later
//
// FindFirstFile fallback — Master mimari Bölüm 5.2A Katman 2 +
// Bölüm 33.2 Katman B (streaming fallback).
//
// Standart Windows API ile recursive scan. Yavaş ama HER ZAMAN çalışır
// (admin gerektirmez, raw volume erişimi yoktur).
//
// İlkeler:
//   * Symlink/junction takip edilmez (Bölüm 33.2'deki follow_links=false).
//   * MAX_DEPTH=50 — pathological derinlik koruması.
//   * BFS sırası: parent her zaman child'dan önce yerleşir, böylece
//     path_to_id sözlüğü tutarlı kalır.
//   * Çıktı `RawMftEntry` ile aynı şemada — `tree::build_tree` doğrudan
//     besleyebilir. Synthetic NodeId'ler 16'dan başlar (sistem rezervi).
//
// Performans (Bölüm 5.4 vs Bölüm 17.2): hedef 1TB için < 45sn (WizTree free
// seviyesi). Bu v0.1 tek-thread. v0.2'de rayon paralel par-dir + lock-free
// id counter.

use crate::error::{Error, Result};
use crate::scan::tree::ROOT_RECORD;
use crate::scan::walk::{MftEntries, ProgressCb, RawMftEntry, ScanProgress};
use crate::scan::NodeId;
use std::collections::{HashMap, VecDeque};
use std::fs::Metadata;
use std::path::PathBuf;
use std::time::{Instant, UNIX_EPOCH};
use tracing::{debug, info, warn};

const FALLBACK_PROGRESS_INTERVAL: u64 = 500;

/// `fs::metadata().modified()` → Unix saniye. Hata veya pre-epoch ise 0.
fn metadata_mtime_unix(metadata: &Metadata) -> i64 {
    match metadata.modified() {
        Ok(t) => match t.duration_since(UNIX_EPOCH) {
            Ok(d) => d.as_secs() as i64,
            Err(_) => 0, // pre-1970 (NTFS'te mümkün), edge case
        },
        Err(_) => 0,
    }
}

pub const MAX_DEPTH: u32 = 50;
const SYNTHETIC_ID_START: u64 = 16;

fn drive_to_root(drive: &str) -> Result<PathBuf> {
    let letter = drive
        .chars()
        .find(|c| c.is_ascii_alphabetic())
        .ok_or_else(|| Error::Scan(format!("Geçersiz sürücü: '{}'", drive)))?
        .to_ascii_uppercase();
    Ok(PathBuf::from(format!(r"{}:\", letter)))
}

/// Geriye uyumlu — progress callback olmadan.
pub fn scan_find_first(drive: &str) -> Result<MftEntries> {
    scan_find_first_with_progress(drive, None)
}

/// BFS yürüyüş. Her dizin için `read_dir` çağırır, child'ları sıraya alır.
/// Symlink ve reparse point'ler atlanır. `progress_cb` her N entry'de bir
/// çağrılır (Bölüm 9.6.5 streaming feedback).
pub fn scan_find_first_with_progress(
    drive: &str,
    progress_cb: Option<ProgressCb<'_>>,
) -> Result<MftEntries> {
    let start = Instant::now();
    let root = drive_to_root(drive)?;
    debug!(root = %root.display(), "FindFirstFile fallback başlıyor");

    let mut entries: Vec<RawMftEntry> = Vec::with_capacity(8192);
    let mut path_to_id: HashMap<PathBuf, NodeId> = HashMap::new();
    path_to_id.insert(root.clone(), ROOT_RECORD);

    let mut next_id: u64 = SYNTHETIC_ID_START;
    let mut queue: VecDeque<(PathBuf, NodeId, u32)> = VecDeque::new();
    queue.push_back((root.clone(), ROOT_RECORD, 0));

    let mut skipped_errors: u64 = 0;
    let mut skipped_symlinks: u64 = 0;
    let mut max_depth_hits: u64 = 0;
    let mut visited_dirs: u64 = 0;

    while let Some((dir, parent_id, depth)) = queue.pop_front() {
        visited_dirs += 1;
        if let Some(cb) = progress_cb {
            if entries.len() as u64 % FALLBACK_PROGRESS_INTERVAL == 0 {
                cb(&ScanProgress {
                    phase: "fallback_walk",
                    visited: entries.len() as u64,
                    total_estimate: 0, // bilinmiyor, BFS
                    in_use: visited_dirs,
                    last_name: dir.display().to_string(),
                    elapsed_ms: start.elapsed().as_millis() as u64,
                });
            }
        }
        if depth >= MAX_DEPTH {
            max_depth_hits += 1;
            continue;
        }

        let read = match std::fs::read_dir(&dir) {
            Ok(r) => r,
            Err(e) => {
                debug!(path = %dir.display(), error = %e, "read_dir başarısız");
                skipped_errors += 1;
                continue;
            }
        };

        for entry_result in read {
            let entry = match entry_result {
                Ok(e) => e,
                Err(_) => {
                    skipped_errors += 1;
                    continue;
                }
            };

            let file_type = match entry.file_type() {
                Ok(ft) => ft,
                Err(_) => {
                    skipped_errors += 1;
                    continue;
                }
            };

            // Symlink ve reparse point atlanır (Bölüm 33.2 follow_links=false)
            if file_type.is_symlink() {
                skipped_symlinks += 1;
                continue;
            }

            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            let id = next_id;
            next_id += 1;

            // metadata: dosya boyutu + mtime için gerekli. symlink_metadata
            // kullanırız ki bilinmeyen reparse point'ler içine girmeyelim.
            let metadata = std::fs::symlink_metadata(&path).ok();
            let is_dir = file_type.is_dir();
            let data_size = if is_dir {
                0
            } else {
                metadata.as_ref().map(|m| m.len()).unwrap_or(0)
            };
            let modified_unix = metadata.as_ref().map(metadata_mtime_unix).unwrap_or(0);

            entries.push(RawMftEntry {
                record_no: id,
                parent_record_no: parent_id,
                name,
                data_size,
                is_dir,
                modified_unix,
            });

            if is_dir {
                path_to_id.insert(path.clone(), id);
                queue.push_back((path, id, depth + 1));
            }
        }
    }

    if skipped_symlinks > 0 {
        warn!(skipped_symlinks, "symlink/reparse point atlandı");
    }
    if max_depth_hits > 0 {
        warn!(max_depth_hits, max_depth = MAX_DEPTH, "max_depth aşıldı");
    }

    let result = MftEntries {
        drive: drive.to_string(),
        volume_path: root.to_string_lossy().to_string(),
        entries,
        skipped_errors,
        elapsed_ms: start.elapsed().as_millis() as u64,
    };
    info!(
        entries = result.entries.len(),
        skipped = result.skipped_errors,
        elapsed_ms = result.elapsed_ms,
        "FindFirstFile fallback tamamlandı"
    );
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drive_root_normalize() {
        assert_eq!(drive_to_root("C").unwrap(), PathBuf::from(r"C:\"));
        assert_eq!(drive_to_root("d:").unwrap(), PathBuf::from(r"D:\"));
        assert!(drive_to_root("").is_err());
    }

    #[test]
    fn scan_empty_temp_dir() {
        let tmp = std::env::temp_dir().join(format!("dspace-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        std::fs::write(tmp.join("a.txt"), b"hello").unwrap();
        std::fs::write(tmp.join("b.dat"), b"1234567890").unwrap();
        std::fs::create_dir(tmp.join("sub")).unwrap();
        std::fs::write(tmp.join("sub").join("inner.bin"), b"xx").unwrap();

        // drive_to_root tek harf bekler; bu test için doğrudan kullanırız.
        let start = Instant::now();
        let mut entries: Vec<RawMftEntry> = Vec::new();
        let mut path_to_id: HashMap<PathBuf, NodeId> = HashMap::new();
        path_to_id.insert(tmp.clone(), ROOT_RECORD);
        let mut next_id = SYNTHETIC_ID_START;
        let mut queue = VecDeque::new();
        queue.push_back((tmp.clone(), ROOT_RECORD, 0u32));

        while let Some((dir, parent_id, depth)) = queue.pop_front() {
            if depth >= MAX_DEPTH {
                continue;
            }
            for entry in std::fs::read_dir(&dir).unwrap() {
                let entry = entry.unwrap();
                let ft = entry.file_type().unwrap();
                if ft.is_symlink() {
                    continue;
                }
                let id = next_id;
                next_id += 1;
                let name = entry.file_name().to_string_lossy().to_string();
                let path = entry.path();
                let is_dir = ft.is_dir();
                let metadata = std::fs::symlink_metadata(&path).unwrap();
                entries.push(RawMftEntry {
                    record_no: id,
                    parent_record_no: parent_id,
                    name,
                    data_size: if is_dir { 0 } else { metadata.len() },
                    is_dir,
                    modified_unix: metadata_mtime_unix(&metadata),
                });
                if is_dir {
                    queue.push_back((path, id, depth + 1));
                }
            }
        }
        let _ = start;

        // a.txt(5), b.dat(10), sub/, inner.bin(2) → 4 entry
        assert_eq!(entries.len(), 4);
        let total: u64 = entries.iter().map(|e| e.data_size).sum();
        assert_eq!(total, 5 + 10 + 2);
        let dir_count = entries.iter().filter(|e| e.is_dir).count();
        assert_eq!(dir_count, 1);

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
