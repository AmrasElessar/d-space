// SPDX-License-Identifier: GPL-3.0-or-later
//
// SQLite veri modeli — Master mimari Bölüm 14.
//
// Konum: %LOCALAPPDATA%\DSpace\db\dspace.sqlite
// Mode: WAL, busy_timeout=5000ms, synchronous=NORMAL.
//
// Migration: rusqlite_migration crate, forward-only, /migrations/
// altında numaralı SQL dosyaları (Bölüm 14.3).
//
// Tablolar (özet, detay spec Bölüm 8.5, 12.5, 6.5.2):
//   * snapshots, snapshot_entries — Time Machine
//   * staging_items — Staging
//   * staging_wal — Cross-volume two-phase commit (v1.4)
//   * permanent_deletes_forensic — Forensic trace
//   * ml_scores — ML safe-delete cache (v2)
//   * settings — kullanıcı ayarları (single source of truth, Bölüm 4.4)

pub const SCHEMA_VERSION: u32 = 1;

pub fn db_filename() -> &'static str {
    "dspace.sqlite"
}

pub fn current_schema_version() -> u32 {
    SCHEMA_VERSION
}
