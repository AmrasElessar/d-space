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

use crate::db::{db_info, open_db, DbInfo, DbState};
use crate::staging::{list_pending, stage, undo, StagedItem};
use crate::error::{Error, Result};
use crate::scan::{
    is_elevated, node_path, pick_strategy, probe_ntfs, scan_to_tree, top_consumers, walk_mft,
    window_query, MftProbe, MftWalkStats, Node, ScanStrategy, ScanSummary, ScanTreeState,
    SortKey, WindowResult,
};
use std::sync::Arc;
use crate::volume::{pre_flight_check, VolumeInfo};
use serde::Serialize;
use tracing::{info, warn};

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

/// Bölüm 33.2 Katman 0 — tarama denemesinden ÖNCE volume statüsü kontrolü.
/// Kullanıcı-uzayı Win32 API'leri, admin gerekmez.
#[tauri::command]
async fn pre_flight_volume(drive: String) -> Result<VolumeInfo> {
    tokio::task::spawn_blocking(move || pre_flight_check(&drive))
        .await
        .map_err(|e| crate::error::Error::Volume(format!("join hatası: {}", e)))?
}

/// Bölüm 5.1–5.3 — raw volume aç, NTFS boot sector parse et, metadata dön.
/// Yönetici yetkisi yoksa ACCESS_DENIED → Error::Scan.
#[tauri::command]
async fn probe_volume(drive: String) -> Result<MftProbe> {
    tokio::task::spawn_blocking(move || probe_ntfs(&drive))
        .await
        .map_err(|e| Error::Scan(format!("join hatası: {}", e)))?
}

/// Bölüm 5.1 + 4.3 Adım 2 — tüm MFT record'larını sırayla gez, sayım + örnek
/// adlar dön. v0.1 tek-thread, hiyerarşi yok (sonraki sprint Bölüm 4.3 Adım 3).
#[tauri::command]
async fn walk_volume(drive: String) -> Result<MftWalkStats> {
    tokio::task::spawn_blocking(move || walk_mft(&drive))
        .await
        .map_err(|e| Error::Scan(format!("join hatası: {}", e)))?
}

/// Bölüm 4.3 Adım 3 + 4.4 — tam MFT entry koleksiyonu + hiyerarşi + agregat.
/// Sonuç `Arc<ScanTree>` Tauri state'inde tutulur (single source of truth).
#[tauri::command]
async fn scan_full(
    drive: String,
    state: tauri::State<'_, ScanTreeState>,
) -> Result<ScanSummary> {
    let drv = drive.clone();
    let (summary, tree) = tokio::task::spawn_blocking(move || scan_to_tree(&drv))
        .await
        .map_err(|e| Error::Scan(format!("join hatası: {}", e)))??;

    let arc_tree = Arc::new(tree);
    {
        let mut slot = state
            .current
            .write()
            .map_err(|e| Error::Scan(format!("rwlock poisoned: {}", e)))?;
        *slot = Some(arc_tree);
    }
    info!(
        drive,
        node_count = summary.node_count,
        gb = summary.total_bytes / 1_073_741_824,
        "scan_full → state güncellendi"
    );
    Ok(summary)
}

/// Bölüm 9.6 ön-örneği — bir düğümün çocuklarını agregat boyuta göre top-N.
#[tauri::command]
fn tree_top_consumers(
    state: tauri::State<'_, ScanTreeState>,
    parent: u64,
    limit: usize,
) -> Result<Vec<Node>> {
    let guard = state
        .current
        .read()
        .map_err(|e| Error::Scan(format!("rwlock poisoned: {}", e)))?;
    let tree = guard
        .as_ref()
        .ok_or_else(|| Error::Scan("scan_full henüz çağrılmadı".into()))?;
    Ok(top_consumers(tree, parent, limit))
}

/// Bölüm 9.6 — viewport-aware pencere sorgusu (filtreli + sıralı + sayfalı).
#[tauri::command]
fn tree_window(
    state: tauri::State<'_, ScanTreeState>,
    parent: u64,
    sort: Option<SortKey>,
    limit: Option<usize>,
    offset: Option<usize>,
    min_size_bytes: Option<u64>,
) -> Result<WindowResult> {
    let guard = state
        .current
        .read()
        .map_err(|e| Error::Scan(format!("rwlock poisoned: {}", e)))?;
    let tree = guard
        .as_ref()
        .ok_or_else(|| Error::Scan("scan_full henüz çağrılmadı".into()))?;
    Ok(window_query(
        tree,
        parent,
        sort.unwrap_or_default(),
        limit.unwrap_or(200),
        offset.unwrap_or(0),
        min_size_bytes,
    ))
}

/// Bir düğümün root'a kadar olan zincirini döner — breadcrumb için.
#[tauri::command]
fn tree_path(state: tauri::State<'_, ScanTreeState>, id: u64) -> Result<Vec<Node>> {
    let guard = state
        .current
        .read()
        .map_err(|e| Error::Scan(format!("rwlock poisoned: {}", e)))?;
    let tree = guard
        .as_ref()
        .ok_or_else(|| Error::Scan("scan_full henüz çağrılmadı".into()))?;
    Ok(node_path(tree, id))
}

/// Bölüm 12.2 — staging: dosyayı `%LOCALAPPDATA%\DSpace\staging\<ts>` altına
/// atomik MOVE eder ve `staging_items` tablosuna kayıt düşer. 24h undo penceresi.
#[tauri::command]
fn stage_path(
    path: String,
    state: tauri::State<'_, DbState>,
) -> Result<StagedItem> {
    let conn = state
        .conn
        .lock()
        .map_err(|e| Error::Db(format!("mutex poisoned: {}", e)))?;
    stage(&path, &conn)
}

/// Bölüm 12.2 — bekleyen tüm staging item'larını listeler (staged_at DESC).
#[tauri::command]
fn list_staging(state: tauri::State<'_, DbState>) -> Result<Vec<StagedItem>> {
    let conn = state
        .conn
        .lock()
        .map_err(|e| Error::Db(format!("mutex poisoned: {}", e)))?;
    list_pending(&conn)
}

/// Bölüm 12.2.4 — staging item'ı orijinal yoluna geri taşır. Conflict varsa
/// hata döner (v0.2'de conflict resolution dialog gelecek).
#[tauri::command]
fn undo_staging(
    id: i64,
    state: tauri::State<'_, DbState>,
) -> Result<String> {
    let conn = state
        .conn
        .lock()
        .map_err(|e| Error::Db(format!("mutex poisoned: {}", e)))?;
    undo(id, &conn)
}

/// Bölüm 14 — DB metadata sorgusu (path, schema_version, journal_mode, ...).
#[tauri::command]
fn get_db_info(state: tauri::State<'_, DbState>) -> Result<DbInfo> {
    let conn = state
        .conn
        .lock()
        .map_err(|e| Error::Db(format!("mutex poisoned: {}", e)))?;
    db_info(&conn)
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

    let db_state = match open_db() {
        Ok(conn) => DbState::new(conn),
        Err(e) => {
            warn!(?e, "DB açılamadı — geçici in-memory ile devam");
            let conn = rusqlite::Connection::open_in_memory()
                .expect("in-memory SQLite açılamadı");
            DbState::new(conn)
        }
    };

    tauri::Builder::default()
        .manage(db_state)
        .manage(ScanTreeState::default())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_app_info,
            check_privilege,
            pre_flight_volume,
            probe_volume,
            walk_volume,
            scan_full,
            tree_top_consumers,
            tree_window,
            tree_path,
            stage_path,
            list_staging,
            undo_staging,
            get_db_info,
        ])
        .run(tauri::generate_context!())
        .expect("D-Space başlatma hatası");
}
