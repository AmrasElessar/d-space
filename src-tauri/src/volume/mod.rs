// SPDX-License-Identifier: GPL-3.0-or-later
//
// Volume Pre-Flight Status Check — Master mimari Bölüm 33.2 (v1.4 fix).
//
// Tarama DENEMESİ yapılmadan ÖNCE volume statüsü sorgulanır. BitLocker
// kilitli sürücüde Katman A/B/C sonsuz recursive deneme yapmasın diye
// Katman 0 olarak eklendi (v1.4 race condition kapatma).

use serde::Serialize;

#[derive(Debug, Serialize)]
pub enum VolumeStatus {
    Ready,
    BitLockerLocked,
    BitLockerSuspended,
    Encrypted { method: String },
    NotMounted,
    AccessDenied,
    NotFormatted,
}

#[derive(Debug, Serialize)]
pub struct VolumeInfo {
    pub drive_letter: String,
    pub file_system: String,
    pub total_bytes: u64,
    pub free_bytes: u64,
    pub status: VolumeStatus,
}
