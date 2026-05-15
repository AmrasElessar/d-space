// SPDX-License-Identifier: GPL-3.0-or-later
//
// 26.5 Cloud Backup Integration — v2 stub.
//
// Faz 2: staging klasöründeki silmeden önce opsiyonel bulut yedek.
// "Sil ama önce buluta yedekle" akışı, premium feature. Mevcut staging
// (Bölüm 12) lokal disk; cloud backup ek katman.
//
// İlke (Bölüm 22.6): bulut yedek opt-in, hiçbir zaman opt-out yapamayan
// karanlık pattern olmaz. Lisans iptal edilirse veri kullanıcının elinde
// kalır (download dahil 90 günlük grace period).

use crate::error::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum CloudProvider {
    /// D-Space'in kendi backup endpoint'i (premium).
    DSpaceCloud,
    /// Bring-your-own: S3-uyumlu (Backblaze B2, Wasabi vb.).
    S3Compatible,
    /// Kullanıcının OneDrive klasörüne kopya (opsiyonel, free).
    OneDriveFolder,
}

#[derive(Debug, Clone, Serialize)]
pub struct CloudUploadResult {
    pub staging_id: i64,
    pub provider: CloudProvider,
    pub remote_id: String,
    pub uploaded_bytes: u64,
    pub elapsed_ms: u64,
}

/// Bölüm 26.5 — cloud backup trait. Streaming upload, hash verify.
pub trait CloudBackupIntegration: Send + Sync {
    fn provider(&self) -> CloudProvider;

    /// Staging item'ı buluta yedekle. Lokal staging dosyası buluta kopyalanır,
    /// `remote_id` döner. Sonraki permanent_delete bu remote_id'yi de
    /// forensic ledger'a yazar (Bölüm 12.5).
    fn upload_staged(&self, staging_id: i64, staged_path: &str) -> Result<CloudUploadResult>;

    /// Buluttan geri yükle. Kullanıcı 24h staging penceresi geçmiş öğeyi
    /// bulutta hâlâ bulabilir (lisans grace period boyunca).
    fn restore_from_cloud(&self, remote_id: &str, dest_path: &str) -> Result<u64>;

    /// Bulut iz silme (forensic-aware; bizim ledger korur).
    fn delete_remote(&self, remote_id: &str) -> Result<()>;
}
