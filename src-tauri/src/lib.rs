// SPDX-License-Identifier: GPL-3.0-or-later
// D-Space — Windows için akıllı disk analiz ve geri kazanım platformu
// Copyright (C) 2026 D-Space contributors
//
// Master mimari: D-Space-Mimari-v1.4.docx (DONDURULDU)
// Modül-bölüm eşlemesi her modülün başındaki yorumda gösterilir.

pub mod db;
pub mod duplicate;
pub mod error;
pub mod locked_file;
pub mod safe_delete;
pub mod scan;
pub mod snapshot;
pub mod staging;
pub mod volume;

use crate::error::Result;
use crate::scan::{is_elevated, pick_strategy, probe_ntfs, MftProbe, ScanStrategy};
use serde::Serialize;
use tracing::info;

#[derive(Debug, Serialize)]
pub struct AppInfo {
    pub name: &'static str,
    pub version: &'static str,
    pub spec_version: &'static str,
    pub spec_status: &'static str,
    pub license: &'static str,
    pub platform: &'static str,
}

#[derive(Debug, Serialize)]
pub struct PrivilegeStatus {
    pub elevated: bool,
    pub strategy: ScanStrategy,
}

impl AppInfo {
    pub const fn current() -> Self {
        Self {
            name: "D-Space",
            version: env!("CARGO_PKG_VERSION"),
            spec_version: "v1.4",
            spec_status: "DONDURULDU",
            license: "GPL-3.0-or-later",
            platform: std::env::consts::OS,
        }
    }
}

#[tauri::command]
fn get_app_info() -> AppInfo {
    AppInfo::current()
}

#[tauri::command]
fn check_privilege() -> PrivilegeStatus {
    let elevated = is_elevated();
    let strategy = pick_strategy();
    info!(elevated, ?strategy, "yetki durumu sorgulandı");
    PrivilegeStatus { elevated, strategy }
}

/// Bölüm 5.1–5.3 — raw volume aç, NTFS boot sector parse et, metadata dön.
/// Yönetici yetkisi yoksa ACCESS_DENIED → Error::Scan.
#[tauri::command]
async fn probe_volume(drive: String) -> Result<MftProbe> {
    tokio::task::spawn_blocking(move || probe_ntfs(&drive))
        .await
        .map_err(|e| crate::error::Error::Scan(format!("join hatası: {}", e)))?
}

fn init_tracing() {
    use tracing_subscriber::{fmt, EnvFilter};

    let filter = EnvFilter::try_from_env("DSPACE_LOG")
        .unwrap_or_else(|_| EnvFilter::new("info,dspace_lib=debug"));

    let _ = fmt()
        .with_env_filter(filter)
        .with_target(true)
        .with_level(true)
        .try_init();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    init_tracing();
    info!(
        version = AppInfo::current().version,
        spec = AppInfo::current().spec_version,
        "D-Space başlatılıyor"
    );

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_app_info,
            check_privilege,
            probe_volume,
        ])
        .run(tauri::generate_context!())
        .expect("D-Space başlatma hatası");
}
