// SPDX-License-Identifier: GPL-3.0-or-later
//
// 26.1 ML Safe-Delete Scorer — v2 stub.
//
// Faz 1 kural motoru (53 kural, `crate::safe_delete::rules`) deterministik.
// Faz 2'de TFLite tier'lı ML scorer eklenir:
//
//   Tier 1: deterministik kural motoru (mevcut) — daima çalışır
//   Tier 2: yerel ML inference (TFLite, küçük model ~5 MB) — async cache
//   Tier 3: cloud inference (premium, opt-in) — büyük model
//
// İlke: ML scan critical path DIŞINDA (Bölüm 6.5.1). SQLite `ml_scores`
// tablosu cache (Bölüm 6.5.2 — schema şimdiden mevcut). mtime değişmedikçe
// yeniden hesaplama yok.

use crate::error::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum InferenceTier {
    /// Bölüm 6.5.1 — deterministik kural motoru.
    Rules,
    /// Bölüm 6.5.1 — yerel TFLite inference.
    LocalMl,
    /// Bölüm 6.5.1 — cloud inference (premium).
    CloudMl,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MlScoreRecord {
    pub path_hash_hex: String,
    pub mtime_unix: i64,
    pub score: u8,
    pub tier: InferenceTier,
    pub computed_at_unix: i64,
}

/// Bölüm 6.5 — ML scorer trait.
///
/// v1.x'te concrete impl `safe_delete::RulesScorer`; tract path opsiyonel
/// `ml-tflite` feature ile. Caller: scan-time path'ten ASLA çağrılmaz; UI
/// drilldown veya idle background job (Bölüm 6.5.3 throttle).
///
/// `Send + Sync` constraint v0.2'de eklenecek — şu an `RulesScorer` SQLite
/// `&Connection` ref tutar (Connection !Sync), Faz 1 tek-thread caller.
/// Background indexing implementation eklendiğinde `Arc<Mutex<Connection>>`
/// pattern'i ile thread-safe yapılır.
pub trait MlSafeDeleteScorer {
    /// Tek dosya skoru. Cache hit ise hızlı, miss ise model inference.
    fn score_one(&self, path: &str, mtime_unix: i64) -> Result<MlScoreRecord>;

    /// Batch — idle background indexing için (Bölüm 6.5.3).
    fn score_batch(&self, paths: &[(&str, i64)]) -> Result<Vec<MlScoreRecord>>;

    /// Cache'ten bilinen skoru oku, miss ise None.
    fn lookup_cache(&self, path_hash: &[u8; 32]) -> Result<Option<MlScoreRecord>>;

    /// İnference tier'ı.
    fn tier(&self) -> InferenceTier;
}
