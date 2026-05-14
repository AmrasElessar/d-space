// SPDX-License-Identifier: GPL-3.0-or-later
//
// Snapshot delta — Master mimari Bölüm 8.6 (Time Machine karşılaştırma).
//
// İki snapshot ID arasında değişen path'leri 4 kategoriye ayırır:
//   * added    — to'da var, from'da yok
//   * removed  — from'da var, to'da yok
//   * grew     — to_size > from_size
//   * shrunk   — to_size < from_size
//
// Her kategori için top-10 entry döner; ancak `total_changed_paths` 4
// kategorinin toplam path sayısını verir. UI streaming delta loader
// (Bölüm 9.6.5) bu özetle aç → genişlet pattern'ini sağlar.

use crate::error::{Error, Result};
use crate::snapshot::SnapshotId;
use rusqlite::{params, Connection};
use serde::Serialize;
use std::collections::HashMap;
use tracing::{debug, info};

const TOP_N: usize = 10;

#[derive(Debug, Clone, Serialize)]
pub struct PathEntry {
    pub path: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeltaEntry {
    pub path: String,
    pub from_size: u64,
    pub to_size: u64,
    pub delta_bytes: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeltaResult {
    pub from_id: SnapshotId,
    pub to_id: SnapshotId,
    pub from_captured_at: i64,
    pub to_captured_at: i64,
    pub net_change_bytes: i64,
    pub total_changed_paths: u64,
    pub added: Vec<PathEntry>,
    pub removed: Vec<PathEntry>,
    pub grew: Vec<DeltaEntry>,
    pub shrunk: Vec<DeltaEntry>,
}

/// İki snapshot'taki entry'leri `HashMap<path_hash → (path, size)>`
/// olarak okur.
fn load_entries(
    conn: &Connection,
    snapshot_id: SnapshotId,
) -> Result<HashMap<Vec<u8>, (String, u64)>> {
    let mut stmt = conn
        .prepare(
            "SELECT path_hash, path, size_bytes FROM snapshot_entries
             WHERE snapshot_id = ?1",
        )
        .map_err(|e| Error::Snapshot(format!("delta prepare: {}", e)))?;

    let rows = stmt
        .query_map(params![snapshot_id], |r| {
            let hash: Vec<u8> = r.get(0)?;
            let path: String = r.get(1)?;
            let size: i64 = r.get(2)?;
            Ok((hash, path, size.max(0) as u64))
        })
        .map_err(|e| Error::Snapshot(format!("delta query: {}", e)))?;

    let mut map = HashMap::new();
    for row in rows {
        let (h, p, s) = row.map_err(|e| Error::Snapshot(format!("delta row: {}", e)))?;
        map.insert(h, (p, s));
    }
    Ok(map)
}

fn snapshot_captured_at(conn: &Connection, snapshot_id: SnapshotId) -> Result<i64> {
    conn.query_row(
        "SELECT captured_at FROM snapshots WHERE id = ?1",
        params![snapshot_id],
        |r| r.get(0),
    )
    .map_err(|e| Error::Snapshot(format!("snapshot id={} bulunamadı: {}", snapshot_id, e)))
}

/// Bölüm 8.6 — iki snapshot arasında set-difference + size-difference
/// hesaplar. Top-10 entry her kategori için döner.
pub fn compute_delta(
    from_id: SnapshotId,
    to_id: SnapshotId,
    conn: &Connection,
) -> Result<DeltaResult> {
    debug!(from_id, to_id, "delta hesaplanıyor");

    let from_captured_at = snapshot_captured_at(conn, from_id)?;
    let to_captured_at = snapshot_captured_at(conn, to_id)?;

    let from_map = load_entries(conn, from_id)?;
    let to_map = load_entries(conn, to_id)?;

    let mut added: Vec<PathEntry> = Vec::new();
    let mut removed: Vec<PathEntry> = Vec::new();
    let mut grew: Vec<DeltaEntry> = Vec::new();
    let mut shrunk: Vec<DeltaEntry> = Vec::new();

    let mut net_change: i64 = 0;

    // to'yu gez: added vs grew/shrunk
    for (hash, (path, to_size)) in &to_map {
        match from_map.get(hash) {
            None => {
                added.push(PathEntry {
                    path: path.clone(),
                    size_bytes: *to_size,
                });
                net_change = net_change.saturating_add(*to_size as i64);
            }
            Some((_from_path, from_size)) => {
                let from_s = *from_size as i64;
                let to_s = *to_size as i64;
                let delta = to_s - from_s;
                if delta > 0 {
                    grew.push(DeltaEntry {
                        path: path.clone(),
                        from_size: *from_size,
                        to_size: *to_size,
                        delta_bytes: delta,
                    });
                    net_change = net_change.saturating_add(delta);
                } else if delta < 0 {
                    shrunk.push(DeltaEntry {
                        path: path.clone(),
                        from_size: *from_size,
                        to_size: *to_size,
                        delta_bytes: delta,
                    });
                    net_change = net_change.saturating_add(delta);
                }
                // delta == 0 → unchanged, sayılmaz.
            }
        }
    }

    // from'u gez: removed
    for (hash, (path, from_size)) in &from_map {
        if !to_map.contains_key(hash) {
            removed.push(PathEntry {
                path: path.clone(),
                size_bytes: *from_size,
            });
            net_change = net_change.saturating_sub(*from_size as i64);
        }
    }

    let total_changed_paths =
        added.len() as u64 + removed.len() as u64 + grew.len() as u64 + shrunk.len() as u64;

    // Sırala + top-N kırp
    added.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));
    added.truncate(TOP_N);

    removed.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));
    removed.truncate(TOP_N);

    grew.sort_by(|a, b| b.delta_bytes.cmp(&a.delta_bytes));
    grew.truncate(TOP_N);

    // shrunk için delta_bytes ASC (en negatif = en çok küçülen önce)
    shrunk.sort_by(|a, b| a.delta_bytes.cmp(&b.delta_bytes));
    shrunk.truncate(TOP_N);

    info!(
        from_id,
        to_id,
        net_change_bytes = net_change,
        total_changed_paths,
        "delta hazır"
    );

    Ok(DeltaResult {
        from_id,
        to_id,
        from_captured_at,
        to_captured_at,
        net_change_bytes: net_change,
        total_changed_paths,
        added,
        removed,
        grew,
        shrunk,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::{params, Connection};

    fn fresh_conn() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        let mig = rusqlite_migration::Migrations::new(vec![
            rusqlite_migration::M::up(include_str!("../db/migrations/0001_initial.sql")),
        ]);
        mig.to_latest(&mut conn).unwrap();
        conn
    }

    fn insert_snapshot(conn: &Connection, captured_at: i64) -> i64 {
        conn.execute(
            "INSERT INTO snapshots
                (volume_id, captured_at, total_size_bytes, file_count, schema_version)
             VALUES ('C:\\', ?1, 0, 0, 1)",
            params![captured_at],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    fn insert_entry(conn: &Connection, snapshot_id: i64, path: &str, size: u64) {
        let hash = blake3::hash(path.as_bytes());
        conn.execute(
            "INSERT INTO snapshot_entries
                (snapshot_id, path_hash, path, size_bytes, modified_at, is_dir)
             VALUES (?1, ?2, ?3, ?4, 0, 1)",
            params![snapshot_id, hash.as_bytes() as &[u8], path, size as i64],
        )
        .unwrap();
    }

    #[test]
    fn delta_four_categories() {
        let conn = fresh_conn();

        // FROM snapshot (t=100)
        let from = insert_snapshot(&conn, 100);
        insert_entry(&conn, from, r"C:\common-stable", 1_000);
        insert_entry(&conn, from, r"C:\going-away", 500);
        insert_entry(&conn, from, r"C:\will-grow", 200);
        insert_entry(&conn, from, r"C:\will-shrink", 800);

        // TO snapshot (t=200)
        let to = insert_snapshot(&conn, 200);
        insert_entry(&conn, to, r"C:\common-stable", 1_000); // unchanged
        insert_entry(&conn, to, r"C:\will-grow", 700); // +500
        insert_entry(&conn, to, r"C:\will-shrink", 300); // -500
        insert_entry(&conn, to, r"C:\brand-new", 900); // added

        let d = compute_delta(from, to, &conn).expect("delta OK");

        assert_eq!(d.from_id, from);
        assert_eq!(d.to_id, to);
        assert_eq!(d.from_captured_at, 100);
        assert_eq!(d.to_captured_at, 200);

        // added: brand-new
        assert_eq!(d.added.len(), 1);
        assert_eq!(d.added[0].path, r"C:\brand-new");
        assert_eq!(d.added[0].size_bytes, 900);

        // removed: going-away
        assert_eq!(d.removed.len(), 1);
        assert_eq!(d.removed[0].path, r"C:\going-away");
        assert_eq!(d.removed[0].size_bytes, 500);

        // grew: will-grow +500
        assert_eq!(d.grew.len(), 1);
        assert_eq!(d.grew[0].path, r"C:\will-grow");
        assert_eq!(d.grew[0].from_size, 200);
        assert_eq!(d.grew[0].to_size, 700);
        assert_eq!(d.grew[0].delta_bytes, 500);

        // shrunk: will-shrink -500
        assert_eq!(d.shrunk.len(), 1);
        assert_eq!(d.shrunk[0].path, r"C:\will-shrink");
        assert_eq!(d.shrunk[0].from_size, 800);
        assert_eq!(d.shrunk[0].to_size, 300);
        assert_eq!(d.shrunk[0].delta_bytes, -500);

        // net: +900 (added) + 500 (grew) - 500 (removed) - 500 (shrunk) = 400
        assert_eq!(d.net_change_bytes, 400);

        // 1+1+1+1 = 4 total changed paths
        assert_eq!(d.total_changed_paths, 4);
    }

    #[test]
    fn delta_top_n_ordering_and_truncation() {
        let conn = fresh_conn();

        let from = insert_snapshot(&conn, 100);
        let to = insert_snapshot(&conn, 200);

        // 15 added — only top 10 by size should remain, descending.
        for i in 0..15 {
            let path = format!("C:\\added-{:02}", i);
            // size = 1000 + i*100, so largest is i=14 → 2400.
            insert_entry(&conn, to, &path, 1000 + (i * 100) as u64);
        }

        let d = compute_delta(from, to, &conn).unwrap();
        assert_eq!(d.added.len(), 10);
        assert_eq!(d.total_changed_paths, 15);
        // İlk eleman en büyük
        assert_eq!(d.added[0].size_bytes, 1000 + 14 * 100);
        // Sıralama DESC
        for w in d.added.windows(2) {
            assert!(w[0].size_bytes >= w[1].size_bytes);
        }
    }

    #[test]
    fn delta_shrunk_ordering_most_negative_first() {
        let conn = fresh_conn();
        let from = insert_snapshot(&conn, 100);
        let to = insert_snapshot(&conn, 200);

        insert_entry(&conn, from, r"C:\a", 1_000);
        insert_entry(&conn, to, r"C:\a", 900); // -100

        insert_entry(&conn, from, r"C:\b", 10_000);
        insert_entry(&conn, to, r"C:\b", 1_000); // -9000

        insert_entry(&conn, from, r"C:\c", 500);
        insert_entry(&conn, to, r"C:\c", 450); // -50

        let d = compute_delta(from, to, &conn).unwrap();
        assert_eq!(d.shrunk.len(), 3);
        assert_eq!(d.shrunk[0].path, r"C:\b"); // -9000 en negatif
        assert_eq!(d.shrunk[1].path, r"C:\a"); // -100
        assert_eq!(d.shrunk[2].path, r"C:\c"); // -50
        assert!(d.shrunk[0].delta_bytes < d.shrunk[1].delta_bytes);
    }

    #[test]
    fn delta_empty_snapshots() {
        let conn = fresh_conn();
        let from = insert_snapshot(&conn, 100);
        let to = insert_snapshot(&conn, 200);

        let d = compute_delta(from, to, &conn).unwrap();
        assert_eq!(d.added.len(), 0);
        assert_eq!(d.removed.len(), 0);
        assert_eq!(d.grew.len(), 0);
        assert_eq!(d.shrunk.len(), 0);
        assert_eq!(d.net_change_bytes, 0);
        assert_eq!(d.total_changed_paths, 0);
    }

    #[test]
    fn delta_unknown_snapshot_id_errors() {
        let conn = fresh_conn();
        let real = insert_snapshot(&conn, 100);
        let err = compute_delta(real, 9999, &conn).unwrap_err();
        match err {
            Error::Snapshot(msg) => assert!(msg.contains("9999")),
            other => panic!("beklenen Snapshot hatası, gelen: {:?}", other),
        }
    }
}
