// SPDX-License-Identifier: GPL-3.0-or-later
//
// Duplicate Detector — dört aşamalı pipeline.
// Master mimari Bölüm 7.
//
// İlkeler:
//   * 0-byte ve <4KB filtre (Bölüm 7.2): kullanıcı `min_duplicate_size_bytes`.
//   * Hash: Blake3 (Bölüm 7.3) — paralel, SIMD, en hızlı dengeli.
//   * Symlink/junction/hardlink inode bazlı (Bölüm 7.2): aynı inode dedup
//     edilmez.
//   * Performans hedefi: 100GB karışık veride <60sn (Bölüm 7.4).

use serde::Serialize;

pub const DEFAULT_MIN_DUP_SIZE: u64 = 4096;

#[derive(Debug, Serialize)]
pub struct DuplicateGroup {
    pub hash_hex: String,
    pub size_bytes: u64,
    pub paths: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct DuplicateStats {
    pub group_count: u64,
    pub redundant_bytes: u64,
    pub elapsed_ms: u64,
}
