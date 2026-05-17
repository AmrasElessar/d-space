// SPDX-License-Identifier: GPL-3.0-or-later
// D-Space — Windows için akıllı disk analiz ve geri kazanım platformu
// Copyright (C) 2026 D-Space contributors
//
// Master mimari: D-Space-Mimari-v1.4.docx (DONDURULDU)
// Modül-bölüm eşlemesi her modülün başındaki yorumda gösterilir.

pub mod db;
pub mod duplicate;
pub mod error;
pub mod index;
pub mod locked_file;
pub mod safe_delete;
pub mod scan;
pub mod snapshot;
pub mod staging;
pub mod v2;
pub mod volume;

use crate::db::{db_info, get_setting, open_db, set_setting, DbInfo, DbState};
use crate::duplicate::{find_duplicates, DuplicateOptions, DuplicateScanResult};
use crate::error::{Error, Result};
use crate::index::{
    enumerate_volume_baseline, index_search as index_search_db, index_status as index_status_db,
    IndexSearchResult, IndexStatus, DEFAULT_BASELINE_BUFFER,
};
use crate::locked_file::{probe_file, LockedFileProbe};
use crate::safe_delete::{
    add_rule as add_user_rule, delete_rule as delete_user_rule, list_active_snapshots,
    list_rules as list_user_rules, toggle_rule as toggle_user_rule, UserPatternType, UserRule,
};
use crate::scan::{
    is_elevated, node_path, pick_strategy, probe_cloud_state, probe_ntfs,
    scan_to_tree_with_progress, top_consumers, walk_mft, window_query, CloudProbe, MftProbe,
    MftWalkStats, Node, ScanProgress, ScanStrategy, ScanSummary, ScanTreeState, SortKey,
    WindowResult,
};
use crate::snapshot::{DeltaResult, SnapshotMeta};
use crate::staging::{
    cleanup_expired, list_expired, list_pending, permanent_delete, recover_wal, stage, undo,
    undo_with_resolution, CleanupReport, ConflictResolution, ExpiredItem, PermanentDeleteResult,
    StagedItem, UndoOutcome, WalRecoveryReport,
};
use crate::volume::{
    list_drives, pre_flight_check, probe_drive_hardware, DriveHardware, VolumeInfo,
};
use serde::Serialize;
use std::sync::Arc;
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

/// Bölüm 33.2 + 15.1 v0.2 — sistemdeki tüm mount edilmiş sürücüler.
/// `GetLogicalDrives` bitmask → her sürücü için `pre_flight_check`. Sol
/// VolumeSidebar UI bunu çağırıp drive listesini render eder.
#[tauri::command]
async fn list_drives_cmd() -> Result<Vec<VolumeInfo>> {
    tokio::task::spawn_blocking(list_drives)
        .await
        .map_err(|e| crate::error::Error::Volume(format!("join hatası: {}", e)))?
}

/// Bir sürücünün donanım profilini döner: bus tipi (NVMe/SATA/USB),
/// medya tipi (SSD/HDD), üretici, model, seri, tipik okuma hızı.
/// Win32 `IOCTL_STORAGE_QUERY_PROPERTY` kullanılır; admin gerekmez.
#[tauri::command]
async fn probe_drive_hardware_cmd(drive: String) -> Result<DriveHardware> {
    let letter = drive
        .chars()
        .find(|c| c.is_ascii_alphabetic())
        .ok_or_else(|| Error::Volume(format!("geçersiz drive: {}", drive)))?;
    tokio::task::spawn_blocking(move || probe_drive_hardware(letter))
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

/// Bölüm 4.3 Adım 3 + 4.4 + 6.4 + 9.6.5 — tam MFT entry koleksiyonu + hiyerarşi +
/// agregat. Bölüm 6.4 user rules önce yüklenir; Bölüm 9.6.5 progress event'leri
/// her N entry'de `scan-progress` olarak emit edilir.
#[tauri::command]
async fn scan_full<R: tauri::Runtime>(
    drive: String,
    window: tauri::WebviewWindow<R>,
    scan_state: tauri::State<'_, ScanTreeState>,
    db_state: tauri::State<'_, DbState>,
) -> Result<ScanSummary> {
    use tauri::Emitter;
    let drv = drive.clone();
    let user_rules = {
        let conn = db_state
            .conn
            .lock()
            .map_err(|e| Error::Db(format!("mutex poisoned: {}", e)))?;
        list_active_snapshots(&conn)?
    };

    // Progress channel: scan thread → emit task. spawn_blocking içinden
    // doğrudan window.emit çağrısı thread-safety açısından güvenli ama
    // mpsc decouple gathering+emit, smooth rate-limiting yapar.
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<ScanProgress>();
    let window_for_emit = window.clone();
    let emit_task = tokio::spawn(async move {
        while let Some(progress) = rx.recv().await {
            let _ = window_for_emit.emit("scan-progress", &progress);
        }
    });

    let (summary, tree) = tokio::task::spawn_blocking(move || {
        let cb: Box<dyn Fn(&ScanProgress) + Send + Sync> = Box::new(move |p| {
            let _ = tx.send(p.clone());
        });
        scan_to_tree_with_progress(&drv, &user_rules, Some(cb.as_ref()))
    })
    .await
    .map_err(|e| Error::Scan(format!("join hatası: {}", e)))??;

    emit_task.abort();
    // Son bir emit — tamamlandı. Sprint 3.7: final partial_tree de göndeririz
    // ki canlı sunburst tarama sonunda tam haritayı (root + 2 seviye + top-200)
    // alıp final state'e geçiş yapsın.
    let final_partial = crate::scan::tree::build_partial_view(&tree, 2, 200);
    let _ = window.emit(
        "scan-progress",
        &ScanProgress {
            phase: "done",
            visited: summary.node_count,
            total_estimate: summary.node_count,
            in_use: summary.node_count,
            last_name: String::new(),
            elapsed_ms: summary.collect_elapsed_ms + summary.build_elapsed_ms,
            partial_tree: Some(final_partial),
        },
    );

    let arc_tree = Arc::new(tree);
    {
        let mut slot = scan_state
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
fn stage_path(path: String, state: tauri::State<'_, DbState>) -> Result<StagedItem> {
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

/// Bölüm 12.3 — WAL recovery raporu (açılışta otomatik çağrılır, bu komut
/// manuel re-run veya UI diagnostic için).
#[tauri::command]
fn run_wal_recovery(state: tauri::State<'_, DbState>) -> Result<WalRecoveryReport> {
    let conn = state
        .conn
        .lock()
        .map_err(|e| Error::Db(format!("mutex poisoned: {}", e)))?;
    recover_wal(&conn)
}

/// Bölüm 12.2.4 — staging item'ı orijinal yoluna geri taşır. Üç sonuç:
/// Restored (hedef boştu), Idempotent (hedef = staged, 4 KB hash match),
/// Conflict (UI dialog → undo_resolve_staging çağrısı).
#[tauri::command]
fn undo_staging(id: i64, state: tauri::State<'_, DbState>) -> Result<UndoOutcome> {
    let conn = state
        .conn
        .lock()
        .map_err(|e| Error::Db(format!("mutex poisoned: {}", e)))?;
    undo(id, &conn)
}

/// Bölüm 12.2.4.2 — conflict dialog'undan dönen kullanıcı seçimini
/// uygula: Overwrite / Rename / KeepBoth / Cancel.
#[tauri::command]
fn undo_resolve_staging(
    id: i64,
    resolution: ConflictResolution,
    state: tauri::State<'_, DbState>,
) -> Result<UndoOutcome> {
    let conn = state
        .conn
        .lock()
        .map_err(|e| Error::Db(format!("mutex poisoned: {}", e)))?;
    undo_with_resolution(id, resolution, &conn)
}

/// Bölüm 12.2.1 — süresi geçmiş staging item'larını listele (expires_at < now).
#[tauri::command]
fn list_expired_staging_cmd(state: tauri::State<'_, DbState>) -> Result<Vec<ExpiredItem>> {
    let conn = state
        .conn
        .lock()
        .map_err(|e| Error::Db(format!("mutex poisoned: {}", e)))?;
    list_expired(&conn)
}

/// Bölüm 12.2.2-2.3 — onaylı toplu cleanup. AUTO_THRESHOLD aşımında dönen
/// rapor `aborted_threshold = true` olur; UI kullanıcıdan açık onay ister.
/// `force=true` "Hepsini sil" onayını işaretler.
#[tauri::command]
fn cleanup_expired_staging_cmd(
    rate_per_sec: Option<u32>,
    force: Option<bool>,
    state: tauri::State<'_, DbState>,
) -> Result<CleanupReport> {
    let mut conn = state
        .conn
        .lock()
        .map_err(|e| Error::Db(format!("mutex poisoned: {}", e)))?;
    let rate = rate_per_sec.unwrap_or(crate::staging::expiry::DEFAULT_CLEANUP_RATE_PER_SEC);
    cleanup_expired(&mut conn, rate, force.unwrap_or(false))
}

/// Bölüm 12.4 — staging item'ı disk'ten kalıcı sil + forensic ledger.
/// `confirm_phrase` orijinal dosya adına (case-insensitive) eşit olmalı.
/// Bu çift onayın **ikinci** adımı; ilk adım UI'da "Sil" butonuna basmak.
#[tauri::command]
fn permanent_delete_cmd(
    id: i64,
    confirm_phrase: String,
    state: tauri::State<'_, DbState>,
) -> Result<PermanentDeleteResult> {
    let mut conn = state
        .conn
        .lock()
        .map_err(|e| Error::Db(format!("mutex poisoned: {}", e)))?;
    permanent_delete(id, &confirm_phrase, &mut conn)
}

/// Bölüm 6.4 — kullanıcı tanımlı kuralları listele.
#[tauri::command]
fn list_user_rules_cmd(state: tauri::State<'_, DbState>) -> Result<Vec<UserRule>> {
    let conn = state
        .conn
        .lock()
        .map_err(|e| Error::Db(format!("mutex poisoned: {}", e)))?;
    list_user_rules(&conn)
}

/// Bölüm 6.4 — yeni kural ekle. Pattern boş olamaz, skor 0-100.
#[tauri::command]
fn add_user_rule_cmd(
    pattern: String,
    pattern_type: UserPatternType,
    score: u8,
    explanation: String,
    state: tauri::State<'_, DbState>,
) -> Result<UserRule> {
    let conn = state
        .conn
        .lock()
        .map_err(|e| Error::Db(format!("mutex poisoned: {}", e)))?;
    add_user_rule(&conn, &pattern, pattern_type, score, &explanation)
}

/// Bölüm 6.4 — kuralı sil.
#[tauri::command]
fn delete_user_rule_cmd(id: i64, state: tauri::State<'_, DbState>) -> Result<()> {
    let conn = state
        .conn
        .lock()
        .map_err(|e| Error::Db(format!("mutex poisoned: {}", e)))?;
    delete_user_rule(&conn, id)
}

/// Bölüm 6.4 — kuralı aç/kapat (silmeden devre dışı bırak).
#[tauri::command]
fn toggle_user_rule_cmd(id: i64, enabled: bool, state: tauri::State<'_, DbState>) -> Result<()> {
    let conn = state
        .conn
        .lock()
        .map_err(|e| Error::Db(format!("mutex poisoned: {}", e)))?;
    toggle_user_rule(&conn, id, enabled)
}

/// Bölüm 14 — settings KV okuma (onboarding flag, kullanıcı tercihleri vs.).
#[tauri::command]
fn get_setting_cmd(key: String, state: tauri::State<'_, DbState>) -> Result<Option<String>> {
    let conn = state
        .conn
        .lock()
        .map_err(|e| Error::Db(format!("mutex poisoned: {}", e)))?;
    get_setting(&conn, &key)
}

/// Bölüm 14 — settings KV upsert.
#[tauri::command]
fn set_setting_cmd(key: String, value: String, state: tauri::State<'_, DbState>) -> Result<()> {
    let conn = state
        .conn
        .lock()
        .map_err(|e| Error::Db(format!("mutex poisoned: {}", e)))?;
    set_setting(&conn, &key, &value)
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

/// Bölüm 8 — mevcut taranmış ScanTree'den bir snapshot yakalar. `scan_full`
/// önce çağrılmış olmalı. Yalnızca dizinler `snapshot_entries`'e yazılır
/// (v0.1 dir-only; Bölüm 8.6 dosya streaming v0.2'de gelir).
#[tauri::command]
fn capture_snapshot(
    scan_state: tauri::State<'_, ScanTreeState>,
    db_state: tauri::State<'_, DbState>,
) -> Result<SnapshotMeta> {
    let tree_arc = {
        let guard = scan_state
            .current
            .read()
            .map_err(|e| Error::Scan(format!("rwlock poisoned: {}", e)))?;
        guard
            .as_ref()
            .ok_or_else(|| Error::Snapshot("scan_full henüz çağrılmadı".into()))?
            .clone()
    };
    let mut conn = db_state
        .conn
        .lock()
        .map_err(|e| Error::Db(format!("mutex poisoned: {}", e)))?;
    crate::snapshot::capture_snapshot(&tree_arc, &mut conn)
}

/// Bölüm 8 — son 50 snapshot'ı captured_at DESC sırasında döner.
#[tauri::command]
fn list_snapshots(db_state: tauri::State<'_, DbState>) -> Result<Vec<SnapshotMeta>> {
    let conn = db_state
        .conn
        .lock()
        .map_err(|e| Error::Db(format!("mutex poisoned: {}", e)))?;
    crate::snapshot::list_snapshots(&conn)
}

/// Bölüm 11.1 — tek dosya için cloud placeholder probe. GetFileAttributesW
/// ile FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS / RECALL_ON_OPEN / REPARSE_POINT
/// bayraklarını okur. spawn_blocking — Win32 senkron çağrı.
#[tauri::command]
async fn probe_cloud_state_cmd(path: String) -> Result<CloudProbe> {
    let p = std::path::PathBuf::from(path);
    tokio::task::spawn_blocking(move || probe_cloud_state(&p))
        .await
        .map_err(|e| Error::Scan(format!("join hatası: {}", e)))?
}

/// Bölüm 34 — tek dosya için locked state + lock owner probe.
/// Yalnızca on-demand çağrılır (Bölüm 34.5.1 hot path izolasyonu). VSS yok,
/// snapshot read v0.2 sprint'inde gelir. spawn_blocking → CreateFileW +
/// Restart Manager senkron Win32 çağrıları.
#[tauri::command]
async fn probe_locked_file_cmd(path: String) -> Result<LockedFileProbe> {
    let p = std::path::PathBuf::from(path);
    tokio::task::spawn_blocking(move || probe_file(&p))
        .await
        .map_err(|e| Error::LockedFile(format!("join hatası: {}", e)))?
}

/// Bölüm 7 — taranmış ScanTree üzerinde duplicate aramayı çalıştırır. `scan_full`
/// önce çağrılmış olmalı. Blake3 streaming hash; tek-thread v0.1. Sonuç:
/// boyut-bucket → hash-bucket grupları, en çok kazandıran önce.
#[tauri::command]
async fn find_duplicates_cmd(
    drive: String,
    min_size_bytes: Option<u64>,
    size_only: Option<bool>,
    state: tauri::State<'_, ScanTreeState>,
) -> Result<DuplicateScanResult> {
    let tree_arc = {
        let guard = state
            .current
            .read()
            .map_err(|e| Error::Scan(format!("rwlock poisoned: {}", e)))?;
        guard
            .as_ref()
            .ok_or_else(|| Error::Duplicate("scan_full henüz çağrılmadı".into()))?
            .clone()
    };
    let letter = drive
        .chars()
        .find(|c| c.is_ascii_alphabetic())
        .ok_or_else(|| Error::Duplicate(format!("Geçersiz sürücü: '{}'", drive)))?;
    let opts = DuplicateOptions {
        min_size_bytes: min_size_bytes.unwrap_or(crate::duplicate::DEFAULT_MIN_DUP_SIZE),
        size_only: size_only.unwrap_or(false),
    };
    // Hash I/O blocking — spawn_blocking ile asenkron.
    tokio::task::spawn_blocking(move || find_duplicates(&tree_arc, letter, opts))
        .await
        .map_err(|e| Error::Duplicate(format!("join hatası: {}", e)))?
}

/// Sprint 3.8 — Discovery #005 / Bölüm 5.6. USN journal index'inden anlık
/// substring araması. Tüm ciltlerden eşleşen ilk `limit` kaydı döner;
/// `full_path` opsiyonel olarak parent zinciri ile çözülür (top-N için
/// yeterli; v0.1 perf hedefi < 50 ms).
#[tauri::command]
fn index_search(
    query: String,
    limit: Option<usize>,
    state: tauri::State<'_, DbState>,
) -> Result<Vec<IndexSearchResult>> {
    let conn = state
        .conn
        .lock()
        .map_err(|e| Error::Db(format!("mutex poisoned: {}", e)))?;
    index_search_db(&conn, &query, limit.unwrap_or(50))
}

/// Sprint 3.8 — Discovery #005. Index durum sorgusu. `drive` opsiyonel:
/// boş geçildiğinde global sayım dönülür. Admin yoksa `mode="needs_admin"`.
#[tauri::command]
fn index_status(drive: Option<String>, state: tauri::State<'_, DbState>) -> Result<IndexStatus> {
    let conn = state
        .conn
        .lock()
        .map_err(|e| Error::Db(format!("mutex poisoned: {}", e)))?;
    let volume_id = drive
        .as_ref()
        .and_then(|d| d.chars().find(|c| c.is_ascii_alphabetic()))
        .map(|c| format!(r"\\.\{}:", c.to_ascii_uppercase()));
    let mut st = index_status_db(&conn, volume_id.as_deref())?;
    if !is_elevated() && st.mode == "idle" {
        // Henüz indeks yok ve admin de yok → UI'a "needs_admin" göster.
        st.mode = "needs_admin".to_string();
    }
    Ok(st)
}

/// Sprint 3.8.1 — baseline enumerate. Admin yoksa `mode="needs_admin"`
/// durumu döner; aksi halde gerçek `FSCTL_ENUM_USN_DATA` walker tüm MFT'yi
/// gezer, `usn_index` tablosuna yazar ve watermark'ı (`usn_watermark`) kaydeder.
#[tauri::command]
async fn index_build(
    drive: String,
    _force: Option<bool>,
    db_state: tauri::State<'_, DbState>,
) -> Result<IndexStatus> {
    let drive_letter = drive
        .chars()
        .find(|c| c.is_ascii_alphabetic())
        .map(|c| c.to_ascii_uppercase())
        .ok_or_else(|| Error::Index(format!("geçersiz drive parametresi: {}", drive)))?;
    let volume_id = format!(r"\\.\{}:", drive_letter);

    if !is_elevated() {
        let conn = db_state
            .conn
            .lock()
            .map_err(|e| Error::Db(format!("mutex poisoned: {}", e)))?;
        let mut st = index_status_db(&conn, Some(&volume_id))?;
        st.mode = "needs_admin".to_string();
        return Ok(st);
    }

    let mut conn = db_state
        .conn
        .lock()
        .map_err(|e| Error::Db(format!("mutex poisoned: {}", e)))?;
    let summary = enumerate_volume_baseline(&volume_id, &mut conn, DEFAULT_BASELINE_BUFFER)?;
    info!(
        volume = %volume_id,
        records = summary.records_seen,
        entries = summary.entries_written,
        batches = summary.batches,
        "USN baseline tamamlandı"
    );
    index_status_db(&conn, Some(&volume_id))
}

/// Bölüm 8.6 — iki snapshot ID arasındaki delta (added/removed/grew/shrunk
/// top-10 + net byte change).
#[tauri::command]
fn snapshot_delta(from: i64, to: i64, db_state: tauri::State<'_, DbState>) -> Result<DeltaResult> {
    let conn = db_state
        .conn
        .lock()
        .map_err(|e| Error::Db(format!("mutex poisoned: {}", e)))?;
    crate::snapshot::compute_delta(from, to, &conn)
}

/// Bölüm 13.1 — system tray ikon + minimum menü. Auto-clean polling
/// (Bölüm 13.2/13.3) v0.2 sprint'inde; bu v0.1 sadece pencere
/// göster/gizle + çıkış.
///
/// İkon: Tauri'nin default app ikonunu kullanır (icon.ico zaten paketli).
/// İleride özel tray ikonu (16x16 mono) eklenebilir.
fn build_tray<R: tauri::Runtime>(app: &tauri::App<R>) -> tauri::Result<()> {
    use tauri::menu::{MenuBuilder, MenuEvent};
    use tauri::tray::TrayIconBuilder;
    use tauri::{Emitter, Manager};

    let menu = MenuBuilder::new(app)
        .text("open", "D-Space'i aç")
        .text("scan_c", "Tara: C:")
        .separator()
        .text("quit", "Çıkış")
        .build()?;

    let Some(icon) = app.default_window_icon().cloned() else {
        warn!("default window icon yok — tray atlandı");
        return Ok(());
    };

    TrayIconBuilder::with_id("dspace-tray")
        .icon(icon)
        .menu(&menu)
        .tooltip("D-Space — Görmek, anlamak, geri kazanmak")
        .show_menu_on_left_click(true)
        .on_menu_event(move |app, event: MenuEvent| {
            match event.id().as_ref() {
                "open" => {
                    if let Some(win) = app.get_webview_window("main") {
                        let _ = win.show();
                        let _ = win.set_focus();
                    }
                }
                "scan_c" => {
                    if let Some(win) = app.get_webview_window("main") {
                        let _ = win.show();
                        let _ = win.set_focus();
                        // Pencereye event yolla; UI yakalayıp runFullScan tetikleyecek
                        let _ = win.emit("tray-scan-request", "C");
                    }
                }
                "quit" => {
                    app.exit(0);
                }
                _ => {}
            }
        })
        .build(app)?;

    info!("system tray hazır");
    Ok(())
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
            let conn = rusqlite::Connection::open_in_memory().expect("in-memory SQLite açılamadı");
            DbState::new(conn)
        }
    };

    // Bölüm 12.3 — startup WAL recovery (crash sonrası BEGIN durumundaki
    // entry'leri ABORTED'a çevir, ortalıkta kalan .dspace_tmp temizle).
    {
        let conn = db_state.conn.lock().expect("DB mutex poisoned");
        match recover_wal(&conn) {
            Ok(report) => {
                if report.scanned > 0 {
                    info!(?report, "açılış WAL recovery");
                }
            }
            Err(e) => warn!(?e, "WAL recovery hatası"),
        }
    }

    tauri::Builder::default()
        .manage(db_state)
        .manage(ScanTreeState::default())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            build_tray(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_app_info,
            check_privilege,
            pre_flight_volume,
            list_drives_cmd,
            probe_drive_hardware_cmd,
            probe_volume,
            walk_volume,
            scan_full,
            tree_top_consumers,
            tree_window,
            tree_path,
            stage_path,
            list_staging,
            undo_staging,
            run_wal_recovery,
            get_db_info,
            capture_snapshot,
            list_snapshots,
            snapshot_delta,
            find_duplicates_cmd,
            probe_locked_file_cmd,
            permanent_delete_cmd,
            undo_resolve_staging,
            get_setting_cmd,
            set_setting_cmd,
            list_expired_staging_cmd,
            cleanup_expired_staging_cmd,
            list_user_rules_cmd,
            add_user_rule_cmd,
            delete_user_rule_cmd,
            toggle_user_rule_cmd,
            probe_cloud_state_cmd,
            index_build,
            index_status,
            index_search,
        ])
        .run(tauri::generate_context!())
        .expect("D-Space başlatma hatası");
}
