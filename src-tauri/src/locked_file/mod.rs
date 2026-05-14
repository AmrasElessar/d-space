// SPDX-License-Identifier: GPL-3.0-or-later
//
// Locked File Handling + VSS Snapshot Pool
// Master mimari Bölüm 34.
//
// İlkeler:
//   * VSS scan critical path DIŞINDA (Bölüm 34.5.1). Scan sırasında
//     kilitli dosya sadece flag konur, içerik atlanır.
//   * Reference counting + lease renewal (Bölüm 34.5.6, v1.4 fix):
//     aktif okuma sırasında snapshot ASLA evict edilmez.
//   * Max 1 aktif snapshot per volume (Windows VSS limiti).
//   * Process owner tespiti (Bölüm 34.4): NtQuerySystemInformation
//     ile handle table okuma — Sysinternals Handle.exe equivalent.

use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct LockOwner {
    pub pid: u32,
    pub process_name: String,
}

#[derive(Debug, Serialize)]
pub enum LockedFileAction {
    SkipContent,
    SnapshotRead,
    UserDrillDown,
}
