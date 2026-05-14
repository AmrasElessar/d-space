// SPDX-License-Identifier: GPL-3.0-or-later
//
// Tarama motoru — Master mimari Bölüm 5 (MFT) + Bölüm 9.6 (lazy viewport
// query) + Bölüm 33 (external/network drive katmanları).
//
// Mimari ilkeler:
//   * Single Source of Truth (Bölüm 4.4): `Arc<ScanTree>` Rust tarafında
//     tek sahip. Vue sadece `NodeId` referansı tutar.
//   * Üç katmanlı yetki stratejisi (Bölüm 5.2A): MFT Service → admin raw
//     volume → FindFirstFile fallback.
//   * Hot-path izolasyonu: VSS scan sırasında ASLA çalışmaz (bkz.
//     locked_file Bölüm 34.5.1).
//
// Bu dosya v0.1 iskelet — gerçek implementasyon Faz 1 sprintlerine yayılır.

use serde::Serialize;

pub type NodeId = u64;

#[derive(Debug, Clone, Serialize)]
pub struct FileNode {
    pub id: NodeId,
    pub parent: Option<NodeId>,
    pub name: String,
    pub size_bytes: u64,
    pub is_dir: bool,
    pub modified_unix: i64,
}

#[derive(Debug, Default, Serialize)]
pub struct ScanTree {
    pub root: Option<NodeId>,
    pub total_size_bytes: u64,
    pub node_count: u64,
}

#[derive(Debug, Serialize)]
pub enum ScanStrategy {
    MftService,
    DirectRawVolume,
    FindFirstFileFallback,
}
