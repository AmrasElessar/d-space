// SPDX-License-Identifier: GPL-3.0-or-later
//
// Staging + Undo + Cross-Volume Two-Phase Commit
// Master mimari Bölüm 12 (12.1–12.5).
//
// İlkeler:
//   * Direkt silme yok. Tüm silme MOVE → staging klasörü.
//   * Undo penceresi default 24 saat, kullanıcı özelleştirir.
//   * Cross-volume kopya: .dspace_tmp + WAL + Blake3 hash verify +
//     atomik rename (Bölüm 12.3 Katman C, v1.4 fix).
//   * Lazy expiry: arka planda otomatik kalıcı silme YOK
//     (Bölüm 12.2.3, Bölüm 22.6 dark pattern yok).
//   * Conflict resolution (Bölüm 12.2.4): üzerine yaz / yeni isim /
//     her ikisini koru / iptal.

pub mod cross_volume;
pub mod ops;
pub mod permanent;
pub mod wal;

pub use cross_volume::{blake3_file, cross_volume_stage_file};
pub use ops::{
    list_pending, stage, undo, undo_with_resolution, ConflictResolution, FileSnapshot, StagedItem,
    UndoOutcome, STAGING_TTL_SECS,
};
pub use permanent::{permanent_delete, PermanentDeleteResult};
pub use wal::{recover_wal, WalRecoveryReport};

use serde::Serialize;

#[derive(Debug, Serialize)]
pub enum FallbackTier {
    Normal,
    RecycleBin,
    CrossVolume,
}

#[derive(Debug, Serialize)]
pub enum WalState {
    Begin,
    Committed,
    Aborted,
}
