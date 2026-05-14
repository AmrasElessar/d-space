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

use serde::Serialize;

pub type SnapshotId = i64;

#[derive(Debug, Serialize)]
pub struct Snapshot {
    pub id: SnapshotId,
    pub volume_id: String,
    pub captured_at_unix: i64,
    pub total_size_bytes: u64,
    pub file_count: u64,
    pub schema_version: u32,
}

#[derive(Debug, Serialize)]
pub struct SnapshotDelta {
    pub path: String,
    pub delta_bytes: i64,
    pub kind: DeltaKind,
}

#[derive(Debug, Serialize)]
pub enum DeltaKind {
    Added,
    Removed,
    Grew,
    Shrunk,
    Unchanged,
}
