// SPDX-License-Identifier: GPL-3.0-or-later
//
// Staging operasyonları — Bölüm 12.
// v0.1 Faz 1: same-volume `fs::rename` (atomik). Cross-volume two-phase
// commit (Bölüm 12.3 Katman C) sonraki sprint'te.

use crate::error::{Error, Result};
use rusqlite::{params, Connection};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, info, warn};

/// Bölüm 12.2 — varsayılan undo penceresi 24 saat.
pub const STAGING_TTL_SECS: i64 = 24 * 3600;

#[derive(Debug, Clone, Serialize)]
pub struct StagedItem {
    pub id: i64,
    pub original_path: String,
    pub staged_path: String,
    pub size_bytes: u64,
    pub staged_at_unix: i64,
    pub expires_at_unix: i64,
    pub is_dir: bool,
    pub fallback_tier: String,
}

fn staging_base() -> Result<PathBuf> {
    let base = dirs::data_local_dir()
        .ok_or_else(|| Error::Staging("data_local_dir bulunamadı".into()))?;
    Ok(base.join("DSpace").join("staging"))
}

pub fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(windows)]
fn first_drive_letter(p: &Path) -> Option<char> {
    use std::path::{Component, Prefix};
    p.components().next().and_then(|c| match c {
        Component::Prefix(prefix) => match prefix.kind() {
            Prefix::Disk(b) | Prefix::VerbatimDisk(b) => Some(b.to_ascii_uppercase() as char),
            _ => None,
        },
        _ => None,
    })
}

#[cfg(not(windows))]
fn first_drive_letter(_p: &Path) -> Option<char> {
    None
}

/// Bölüm 12.2 — kullanıcı seçimini staging klasörüne atomik MOVE eder.
/// Cross-volume henüz desteklenmez (Bölüm 12.3 Katman C v0.2).
pub fn stage(path: &str, conn: &Connection) -> Result<StagedItem> {
    let src = PathBuf::from(path);
    if !src.exists() {
        return Err(Error::Staging(format!("Bulunamadı: {}", path)));
    }

    let metadata = fs::metadata(&src)
        .map_err(|e| Error::Staging(format!("metadata '{}': {}", path, e)))?;
    let is_dir = metadata.is_dir();
    let size_bytes = if is_dir { 0 } else { metadata.len() };

    let staged_at = now_unix();
    let dest_dir = staging_base()?.join(staged_at.to_string());
    fs::create_dir_all(&dest_dir).map_err(|e| Error::Staging(format!("mkdir: {}", e)))?;

    let file_name = src
        .file_name()
        .ok_or_else(|| Error::Staging("filename eksik".into()))?;
    let mut dest = dest_dir.join(file_name);

    // Collision avoidance — aynı saniye içinde aynı isim olabilir
    let mut suffix = 1u32;
    while dest.exists() {
        dest = dest_dir.join(format!(
            "{:03}_{}",
            suffix,
            file_name.to_string_lossy()
        ));
        suffix += 1;
        if suffix > 999 {
            return Err(Error::Staging("Çakışma sayısı limitini aştı".into()));
        }
    }

    // Cross-volume kontrol (Bölüm 12.3) — two-phase commit yoluna dispatch
    let src_drive = first_drive_letter(&src);
    let dest_drive = first_drive_letter(&dest);
    let is_cross = src_drive.is_some() && dest_drive.is_some() && src_drive != dest_drive;

    if is_cross {
        debug!(
            src = %src.display(),
            dest = %dest.display(),
            "cross-volume tespit edildi → two-phase commit"
        );
        let target_vol = dest_drive
            .map(|c| format!("{}:", c))
            .unwrap_or_else(|| "?".into());
        return crate::staging::cross_volume::cross_volume_stage_file(
            &src,
            &dest,
            &target_vol,
            size_bytes,
            is_dir,
            conn,
        );
    }

    debug!(src = %src.display(), dest = %dest.display(), "same-volume fs::rename");
    fs::rename(&src, &dest).map_err(|e| Error::Staging(format!("rename: {}", e)))?;

    let expires_at = staged_at + STAGING_TTL_SECS;
    conn.execute(
        "INSERT INTO staging_items
            (original_path, staged_path, size_bytes, staged_at, expires_at,
             is_dir, reason, fallback_tier)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL, 'normal')",
        params![
            path,
            dest.to_string_lossy(),
            size_bytes as i64,
            staged_at,
            expires_at,
            is_dir as i64,
        ],
    )
    .map_err(|e| Error::Staging(format!("DB insert: {}", e)))?;

    let id = conn.last_insert_rowid();
    info!(
        id,
        size = size_bytes,
        is_dir,
        original = %path,
        staged = %dest.display(),
        "stage başarılı"
    );

    Ok(StagedItem {
        id,
        original_path: path.to_string(),
        staged_path: dest.to_string_lossy().to_string(),
        size_bytes,
        staged_at_unix: staged_at,
        expires_at_unix: expires_at,
        is_dir,
        fallback_tier: "normal".into(),
    })
}

/// DB'deki tüm bekleyen (henüz silinmemiş) staging item'larını döner.
/// staged_at azalan sırada, max 200 (Bölüm 9.6 limit ile uyumlu).
pub fn list_pending(conn: &Connection) -> Result<Vec<StagedItem>> {
    let mut stmt = conn
        .prepare(
            "SELECT id, original_path, staged_path, size_bytes, staged_at,
                    expires_at, is_dir, fallback_tier
             FROM staging_items
             ORDER BY staged_at DESC
             LIMIT 200",
        )
        .map_err(|e| Error::Staging(format!("prepare: {}", e)))?;

    let rows = stmt
        .query_map([], |r| {
            Ok(StagedItem {
                id: r.get(0)?,
                original_path: r.get(1)?,
                staged_path: r.get(2)?,
                size_bytes: r.get::<_, i64>(3)? as u64,
                staged_at_unix: r.get(4)?,
                expires_at_unix: r.get(5)?,
                is_dir: r.get::<_, i64>(6)? != 0,
                fallback_tier: r.get(7)?,
            })
        })
        .map_err(|e| Error::Staging(format!("query: {}", e)))?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| Error::Staging(format!("row: {}", e)))?);
    }
    Ok(items)
}

/// Bölüm 12.2.4 — undo işlemi. Hedef yolda dosya varsa conflict error döner
/// (v0.1: full conflict resolution dialog yok, sonraki sprint UI'de).
pub fn undo(id: i64, conn: &Connection) -> Result<String> {
    let (original, staged): (String, String) = conn
        .query_row(
            "SELECT original_path, staged_path FROM staging_items WHERE id = ?1",
            params![id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|e| Error::Staging(format!("undo lookup id={}: {}", id, e)))?;

    let src = Path::new(&staged);
    let dest = Path::new(&original);

    if !src.exists() {
        return Err(Error::Staging(format!(
            "Staging dosyası bulunamadı: {}",
            staged
        )));
    }

    // Bölüm 12.2.4.1 — conflict detection
    if dest.exists() {
        warn!(
            dest = %dest.display(),
            "undo conflict — full resolution v0.2'de"
        );
        return Err(Error::Staging(format!(
            "Hedef yolda zaten dosya var: {} \
             — v0.2'de conflict resolution dialog (üzerine yaz / yeni isim / \
             her ikisini koru / iptal). Şimdilik manuel: hedefi taşıyın ve \
             tekrar deneyin.",
            original
        )));
    }

    // Parent dir kayıp olabilir (kullanıcı silmiş olabilir)
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| Error::Staging(format!("parent mkdir: {}", e)))?;
    }

    fs::rename(src, dest)
        .map_err(|e| Error::Staging(format!("undo rename: {}", e)))?;

    conn.execute("DELETE FROM staging_items WHERE id = ?1", params![id])
        .map_err(|e| Error::Staging(format!("DB delete: {}", e)))?;

    info!(id, original = %original, "undo başarılı");
    Ok(original)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_db;

    fn unique_tmp(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "dspace-stage-{}-{}-{}",
            std::process::id(),
            now_unix(),
            name
        ))
    }

    #[test]
    fn stage_and_undo_roundtrip() {
        // Bu test gerçek DB ve dosya sistemi kullanır (in-memory connection +
        // temp dir). Same-volume rename yapar.
        let conn = Connection::open_in_memory().unwrap();
        let migrations = rusqlite_migration::Migrations::new(vec![
            rusqlite_migration::M::up(include_str!("../db/migrations/0001_initial.sql")),
        ]);
        let mut conn = conn;
        migrations.to_latest(&mut conn).unwrap();

        // Çalışma alanı: gerçek FS, geçici dizinler. Kaynak ve staging aynı
        // volume'da olmalı (TEMP).
        let work_root = unique_tmp("work");
        fs::create_dir_all(&work_root).unwrap();
        let src = work_root.join("doomed.txt");
        fs::write(&src, b"deletable content").unwrap();
        assert!(src.exists());

        // Staging base'ı zaten LOCALAPPDATA — bu test üzerine yazılırsa
        // gerçek staging klasörü kullanılır; sonra temizleriz.
        let staged = stage(src.to_str().unwrap(), &conn).expect("stage başarılı");
        assert!(!src.exists(), "kaynak taşınmış olmalı");
        assert!(Path::new(&staged.staged_path).exists());

        // Undo
        let restored = undo(staged.id, &conn).expect("undo başarılı");
        assert_eq!(restored, src.to_string_lossy());
        assert!(src.exists(), "kaynak geri gelmiş olmalı");
        assert!(!Path::new(&staged.staged_path).exists());

        // DB'de staging_item silinmiş olmalı
        let pending = list_pending(&conn).unwrap();
        assert!(pending.iter().all(|p| p.id != staged.id));

        // Temizlik
        let _ = fs::remove_dir_all(&work_root);
    }

    #[test]
    fn list_pending_orders_by_staged_at_desc() {
        let mut conn = Connection::open_in_memory().unwrap();
        let migrations = rusqlite_migration::Migrations::new(vec![
            rusqlite_migration::M::up(include_str!("../db/migrations/0001_initial.sql")),
        ]);
        migrations.to_latest(&mut conn).unwrap();

        // Manuel insert (gerçek FS dokunmadan)
        let now = now_unix();
        conn.execute(
            "INSERT INTO staging_items
             (original_path, staged_path, size_bytes, staged_at, expires_at, is_dir, fallback_tier)
             VALUES ('/a', '/s/a', 10, ?1, ?2, 0, 'normal'),
                    ('/b', '/s/b', 20, ?3, ?4, 0, 'normal'),
                    ('/c', '/s/c', 30, ?5, ?6, 0, 'normal')",
            params![
                now - 100, now - 100 + 86400,
                now - 50,  now - 50  + 86400,
                now,        now + 86400,
            ],
        ).unwrap();

        let items = list_pending(&conn).unwrap();
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].original_path, "/c");
        assert_eq!(items[1].original_path, "/b");
        assert_eq!(items[2].original_path, "/a");
    }

    #[allow(dead_code)]
    fn _silence_unused_open_db_import() {
        let _ = open_db;
    }
}
