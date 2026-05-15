// SPDX-License-Identifier: GPL-3.0-or-later
//
// 26.2 Network Share Scanner — v2 stub.
//
// Faz 1: network drive'lar `volume::pre_flight_check` ile drive_kind=Remote
// olarak işaretlenir, kullanıcı uyarı görür (Bölüm 33.3). Tarama
// FindFirstFile fallback ile yapılır — yavaş.
//
// Faz 2: UNC path için özel scanner. MFT olmadığı için stream-based,
// chunked listing + cache. SMB üzerinden bandwidth-aware.

use crate::error::Result;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct NetworkScanResult {
    pub unc_path: String,
    pub file_count: u64,
    pub dir_count: u64,
    pub total_bytes: u64,
    pub bandwidth_used_bytes: u64,
    pub elapsed_ms: u64,
    pub partial: bool,
}

/// Bölüm 26.2 + 33.3 — UNC path scanner.
pub trait NetworkShareScanner: Send + Sync {
    /// Network share'i tara. `bandwidth_cap_bps` 0 ise sınırsız.
    /// Bölüm 33.3 ilkesi: kullanıcı network maliyetini bilmeli, partial
    /// result her zaman kabul edilir (timeout veya cap aşımı).
    fn scan_unc(&self, unc_path: &str, bandwidth_cap_bps: u64) -> Result<NetworkScanResult>;
}
