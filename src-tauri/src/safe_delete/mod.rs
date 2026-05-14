// SPDX-License-Identifier: GPL-3.0-or-later
//
// Safe-to-Delete zeka katmanı.
// Master mimari Bölüm 6.
//
// Faz 1: deklaratif kural motoru (50+ dahili kural).
// Faz 2 (v1.4 spec, kod v2): TFLite tier'lı ML scorer (Bölüm 6.5).
//
// İlkeler:
//   * Skor 0-100, renk: 0-30 kırmızı, 31-60 sarı, 61-85 yeşil, 86-100 mavi.
//   * Inference ASLA scan critical path'inde değil (Bölüm 6.5.1).
//   * ML cache (Bölüm 6.5.2): mtime değişmezse yeniden hesaplanmaz.
//   * Battery + idle throttle (Bölüm 6.5.3).
//   * v2 stub trait (Bölüm 26.1).

use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ScoredFile {
    pub path: String,
    pub score: u8, // 0..=100
    pub matched_rules: Vec<RuleMatch>,
}

#[derive(Debug, Serialize)]
pub struct RuleMatch {
    pub rule_id: String,
    pub explanation: String,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub enum ScoreTier {
    Danger,    // 0-30  kırmızı
    Caution,   // 31-60 sarı
    Likely,    // 61-85 yeşil
    Cache,     // 86-100 mavi
}

impl ScoreTier {
    pub fn from_score(score: u8) -> Self {
        match score {
            0..=30 => Self::Danger,
            31..=60 => Self::Caution,
            61..=85 => Self::Likely,
            _ => Self::Cache,
        }
    }
}
