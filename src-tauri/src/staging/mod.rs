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

use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct StagedItem {
    pub id: i64,
    pub original_path: String,
    pub staged_path: String,
    pub size_bytes: u64,
    pub staged_at_unix: i64,
    pub expires_at_unix: i64,
    pub fallback_tier: FallbackTier,
}

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
