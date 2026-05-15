// SPDX-License-Identifier: GPL-3.0-or-later
//
// Duplicate Detector — dört aşamalı pipeline.
// Master mimari Bölüm 7.
//
// İlkeler:
//   * 0-byte ve <4 KB filtre (Bölüm 7.2): kullanıcı `min_size_bytes`.
//   * Hash: Blake3 (Bölüm 7.3) — paralel, SIMD, en hızlı dengeli.
//   * Symlink/junction/hardlink inode bazlı (Bölüm 7.2): aynı NodeId iki kez
//     sayılmaz (ScanTree HashMap garantisi — v0.2'de symlink/reparse açık eleme).
//   * Performans hedefi: 100 GB karışık veride <60 sn (Bölüm 7.4).
//
// v0.1 sınırı: tek-thread hash. v0.2'de rayon + head-hash prefilter.

pub mod scan;

pub use scan::{find_duplicates, DuplicateOptions, DEFAULT_MIN_DUP_SIZE};

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct DuplicateGroup {
    /// Tam Blake3 hash hex (64 char) ya da `size_only` modunda "size:<N>".
    pub hash_hex: String,
    pub size_bytes: u64,
    pub paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DuplicateStats {
    pub group_count: u64,
    pub redundant_bytes: u64,
    pub elapsed_ms: u64,
}

/// Bölüm 7 — `find_duplicates` Tauri komutu cevabı.
#[derive(Debug, Clone, Serialize)]
pub struct DuplicateScanResult {
    pub drive_letter: String,
    pub scanned_files: u64,
    pub filtered_small: u64,
    pub candidate_pairs: u64,
    pub hash_errors: u64,
    pub groups: Vec<DuplicateGroup>,
    pub stats: DuplicateStats,
}
