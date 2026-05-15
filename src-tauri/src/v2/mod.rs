// SPDX-License-Identifier: GPL-3.0-or-later
//
// v2 Altyapı Rezervasyonları — Master mimari Bölüm 26.
//
// Bu modül **henüz uygulanmamış** ama API kontratı v1.x boyunca stabil
// kalacak trait'leri içerir. Amaç: v2'ye geçişte breaking change'i
// minimize etmek, mevcut kodu bu trait'ler etrafında konumlandırmak.
//
// Şu an hiçbir trait'in concrete implementation'ı yok — `unimplemented!()`
// veya feature-gated. v2 sprint'lerinde gerçek implementasyon eklenecek.
//
// İçerik:
//   * 26.1 MlSafeDeleteScorer — TFLite tier'lı ML scorer (mevcut kural
//           motorunun üzerine eklenir)
//   * 26.2 NetworkShareScanner — UNC path için scan stratejisi
//   * 26.3 CrossPlatformVolumeReader — Linux/macOS desteği için trait
//   * 26.4 Plugin — kapalı kutu eklenti sistemi
//   * 26.5 CloudBackupIntegration — staging için bulut yedek

pub mod cloud_backup;
pub mod ml_scorer;
pub mod network_scanner;
pub mod plugin;
pub mod volume_reader;

pub use cloud_backup::{CloudBackupIntegration, CloudProvider, CloudUploadResult};
pub use ml_scorer::{InferenceTier, MlSafeDeleteScorer, MlScoreRecord};
pub use network_scanner::{NetworkScanResult, NetworkShareScanner};
pub use plugin::{Plugin, PluginCapabilities, PluginContext};
pub use volume_reader::{CrossPlatformVolumeReader, VolumeBackend};
