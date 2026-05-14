// SPDX-License-Identifier: GPL-3.0-or-later
//
// Time Machine — günlük metadata snapshot sistemi.
// Master mimari Bölüm 8.
//
// İlkeler:
//   * Snapshot başına ~1 MB (sıkıştırılmış), 90 gün varsayılan history.
//   * Delta storage: aynı path için sadece değişen alanlar tutulur.
//   * Saklama politikası (Bölüm 8.4): free 7 gün, premium 365 gün.
//   * Streaming delta loader (Bölüm 9.6.5): iki tam ağaç değil chunk
//     event'i ile UI'a iletilir.

pub mod capture;
pub mod delta;

pub use capture::{capture_snapshot, list_snapshots, SnapshotMeta};
pub use delta::{compute_delta, DeltaEntry, DeltaResult, PathEntry};

/// Snapshot tablosundaki PRIMARY KEY tipiyle uyumlu — i64.
pub type SnapshotId = i64;
