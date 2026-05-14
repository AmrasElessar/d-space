// SPDX-License-Identifier: GPL-3.0-or-later
//
// ScanTree builder — Bölüm 4.3 Adım 3 + Bölüm 4.4 Single Source of Truth.
//
// Raw MFT entry'lerinden:
//   1. Düğüm sözlüğü (`HashMap<NodeId, Node>`)
//   2. Çocuk listesi (`HashMap<NodeId, Vec<NodeId>>`)
//   3. Walk-up aggregate boyut hesabı (her dosya boyutu üst klasörlerine eklenir)
//
// Sonuç `Arc<ScanTree>` olarak Tauri state'inde tutulur. Vue hiçbir zaman
// tam ağaca sahip olmaz — sadece `tree_summary` ve `top_consumers` window
// query'leri ile çalışır (Bölüm 9.6 lazy viewport).

use crate::scan::find_first::scan_find_first;
use crate::scan::privilege::is_elevated;
use crate::scan::walk::{collect_mft_entries, RawMftEntry};
use crate::scan::{NodeId, ScanStrategy};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tracing::{debug, info};

/// NTFS root directory MFT record number.
/// `KnownNtfsFileRecordNumber::RootDirectory = 5`.
pub const ROOT_RECORD: NodeId = 5;
const MAX_PARENT_HOPS: u32 = 256;

#[derive(Debug, Clone, Serialize)]
pub struct Node {
    pub id: NodeId,
    pub parent: Option<NodeId>,
    pub name: String,
    pub data_size: u64,
    pub aggregate_size: u64,
    pub is_dir: bool,
}

#[derive(Debug, Serialize)]
pub struct ScanTree {
    pub volume_id: String,
    pub root_id: NodeId,
    pub nodes: HashMap<NodeId, Node>,
    pub children: HashMap<NodeId, Vec<NodeId>>,
    pub total_bytes: u64,
    pub file_count: u64,
    pub dir_count: u64,
    pub built_at_unix: i64,
    pub build_elapsed_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScanSummary {
    pub drive: String,
    pub volume_id: String,
    pub strategy: ScanStrategy,
    pub root_id: NodeId,
    pub node_count: u64,
    pub file_count: u64,
    pub dir_count: u64,
    pub total_bytes: u64,
    pub collect_elapsed_ms: u64,
    pub build_elapsed_ms: u64,
}

/// Raw MFT entry'lerinden tam ScanTree kurar:
/// 1. Düğümleri yerleştirir, ROOT_RECORD sentetik garanti.
/// 2. Çocuk listelerini oluşturur (orphan'lar root altına).
/// 3. Walk-up agregat: her dosya boyutu üst klasörlerine eklenir.
pub fn build_tree(volume_id: String, raw: Vec<RawMftEntry>) -> ScanTree {
    let start = Instant::now();
    let count = raw.len();

    let mut nodes: HashMap<NodeId, Node> = HashMap::with_capacity(count + 1);
    let mut children: HashMap<NodeId, Vec<NodeId>> = HashMap::with_capacity(count / 8 + 1);

    // 1. Düğüm yerleştirme — self-cycle (record == parent) None.
    for e in &raw {
        let parent_opt = if e.parent_record_no == e.record_no {
            None
        } else {
            Some(e.parent_record_no)
        };
        nodes.insert(
            e.record_no,
            Node {
                id: e.record_no,
                parent: parent_opt,
                name: e.name.clone(),
                data_size: e.data_size,
                aggregate_size: e.data_size,
                is_dir: e.is_dir,
            },
        );
    }

    // Root sentetik düğüm garantisi.
    nodes.entry(ROOT_RECORD).or_insert(Node {
        id: ROOT_RECORD,
        parent: None,
        name: "<volume root>".into(),
        data_size: 0,
        aggregate_size: 0,
        is_dir: true,
    });

    // 2. Orphan parent rewiring: bilinmeyen parent → ROOT_RECORD.
    //    Aynı zamanda ROOT_RECORD'un kendi parent'ını None tutar.
    let known: std::collections::HashSet<NodeId> = nodes.keys().copied().collect();
    for node in nodes.values_mut() {
        if node.id == ROOT_RECORD {
            node.parent = None;
            continue;
        }
        match node.parent {
            Some(p) if !known.contains(&p) => {
                node.parent = Some(ROOT_RECORD);
            }
            _ => {}
        }
    }

    // 3. Çocuk listesi — düzeltilmiş parent pointer'lardan oluşturulur.
    for n in nodes.values() {
        if let Some(p) = n.parent {
            children.entry(p).or_default().push(n.id);
        }
    }

    // 4. Sayım — sadece raw input'a göre, sentetik root sayılmaz.
    let mut total_bytes = 0u64;
    let mut file_count = 0u64;
    let mut dir_count = 0u64;
    for e in &raw {
        if e.is_dir {
            dir_count += 1;
        } else {
            file_count += 1;
            total_bytes = total_bytes.saturating_add(e.data_size);
        }
    }

    // 5. Walk-up aggregate. İki aşama (borrow checker):
    //    a) Snapshot (id, parent, data_size)
    //    b) Walk-up + mut update
    let snapshot: Vec<(NodeId, Option<NodeId>, u64)> = nodes
        .iter()
        .map(|(&id, n)| (id, n.parent, n.data_size))
        .collect();

    for (id, parent, size) in snapshot {
        if size == 0 {
            continue;
        }
        let mut cur = parent;
        let mut hops = 0u32;
        while let Some(p) = cur {
            if hops >= MAX_PARENT_HOPS || p == id {
                break;
            }
            match nodes.get_mut(&p) {
                Some(pn) => {
                    pn.aggregate_size = pn.aggregate_size.saturating_add(size);
                    cur = pn.parent;
                }
                None => break,
            }
            hops += 1;
        }
    }

    let build_elapsed_ms = start.elapsed().as_millis() as u64;
    let built_at_unix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    info!(
        nodes = nodes.len(),
        files = file_count,
        dirs = dir_count,
        total_gb = total_bytes / 1_073_741_824,
        elapsed_ms = build_elapsed_ms,
        "ScanTree kuruldu"
    );

    ScanTree {
        volume_id,
        root_id: ROOT_RECORD,
        nodes,
        children,
        total_bytes,
        file_count,
        dir_count,
        built_at_unix,
        build_elapsed_ms,
    }
}

/// Belirli bir düğümün çocuklarını agregat boyuta göre azalan sırada N adet döner.
pub fn top_consumers(tree: &ScanTree, parent: NodeId, limit: usize) -> Vec<Node> {
    let Some(child_ids) = tree.children.get(&parent) else {
        return Vec::new();
    };
    let mut out: Vec<Node> = child_ids
        .iter()
        .filter_map(|id| tree.nodes.get(id).cloned())
        .collect();
    out.sort_by(|a, b| b.aggregate_size.cmp(&a.aggregate_size));
    out.truncate(limit);
    out
}

/// `collect_mft_entries` + `build_tree` zincirleyen MFT-only API.
pub fn scan_to_tree_mft(drive: &str) -> crate::error::Result<(ScanSummary, ScanTree)> {
    debug!(drive, "MFT full scan başlıyor");
    let collected = collect_mft_entries(drive)?;
    chain_into_summary(drive, ScanStrategy::DirectRawVolume, collected)
}

/// `scan_find_first` + `build_tree` zincirleyen FindFirstFile fallback API.
pub fn scan_to_tree_fallback(drive: &str) -> crate::error::Result<(ScanSummary, ScanTree)> {
    debug!(drive, "FindFirstFile fallback scan başlıyor");
    let collected = scan_find_first(drive)?;
    chain_into_summary(drive, ScanStrategy::FindFirstFileFallback, collected)
}

/// Bölüm 5.2A — yetkiye göre otomatik strateji seçimi. MFT denenir, başarısız
/// olursa fallback'e düşer (Bölüm 33.2 Katman A → Katman B pattern).
pub fn scan_to_tree(drive: &str) -> crate::error::Result<(ScanSummary, ScanTree)> {
    if is_elevated() {
        debug!("elevated process — MFT yolu denenecek");
        match scan_to_tree_mft(drive) {
            Ok(r) => return Ok(r),
            Err(e) => {
                tracing::warn!(?e, "MFT path başarısız → fallback");
            }
        }
    } else {
        debug!("elevated değil — fallback yolu");
    }
    scan_to_tree_fallback(drive)
}

fn chain_into_summary(
    drive: &str,
    strategy: ScanStrategy,
    collected: crate::scan::walk::MftEntries,
) -> crate::error::Result<(ScanSummary, ScanTree)> {
    let collect_elapsed_ms = collected.elapsed_ms;
    let volume_id = collected.volume_path.clone();
    let tree = build_tree(volume_id.clone(), collected.entries);
    let summary = ScanSummary {
        drive: drive.to_string(),
        volume_id,
        strategy,
        root_id: tree.root_id,
        node_count: tree.nodes.len() as u64,
        file_count: tree.file_count,
        dir_count: tree.dir_count,
        total_bytes: tree.total_bytes,
        collect_elapsed_ms,
        build_elapsed_ms: tree.build_elapsed_ms,
    };
    Ok((summary, tree))
}

/// Tauri yönetilen state — opsiyonel Arc<ScanTree>. Tek yazıcı, çok okur.
pub struct ScanTreeState {
    pub current: RwLock<Option<Arc<ScanTree>>>,
}

impl Default for ScanTreeState {
    fn default() -> Self {
        Self {
            current: RwLock::new(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(record_no: u64, parent: u64, name: &str, size: u64, dir: bool) -> RawMftEntry {
        RawMftEntry {
            record_no,
            parent_record_no: parent,
            name: name.into(),
            data_size: size,
            is_dir: dir,
        }
    }

    #[test]
    fn aggregate_two_level() {
        // root(5)
        //   docs(100, dir, aggregate 600)
        //     a.txt(101, 200)
        //     b.txt(102, 400)
        //   c.bin(103, 50, aggregate just 50)
        let raw = vec![
            r(100, 5, "docs", 0, true),
            r(101, 100, "a.txt", 200, false),
            r(102, 100, "b.txt", 400, false),
            r(103, 5, "c.bin", 50, false),
        ];
        let tree = build_tree("vol".into(), raw);
        assert_eq!(tree.file_count, 3);
        assert_eq!(tree.dir_count, 1);
        assert_eq!(tree.total_bytes, 650);
        assert_eq!(tree.nodes.get(&100).unwrap().aggregate_size, 600);
        assert_eq!(tree.nodes.get(&5).unwrap().aggregate_size, 650);
        // top_consumers root → docs ilk
        let top = top_consumers(&tree, 5, 10);
        assert_eq!(top[0].id, 100);
        assert_eq!(top[1].id, 103);
    }

    #[test]
    fn orphan_attached_to_root() {
        // parent 999 sözlükte yok → root altı
        let raw = vec![r(200, 999, "lonely.dat", 128, false)];
        let tree = build_tree("vol".into(), raw);
        let root_children = tree.children.get(&ROOT_RECORD).unwrap();
        assert!(root_children.contains(&200));
        assert_eq!(tree.nodes.get(&5).unwrap().aggregate_size, 128);
    }

    #[test]
    fn cycle_protection() {
        // self-referencing entry
        let raw = vec![r(300, 300, "weird", 64, false)];
        let tree = build_tree("vol".into(), raw);
        assert_eq!(tree.total_bytes, 64);
        assert_eq!(tree.nodes.get(&300).unwrap().parent, None);
    }
}
