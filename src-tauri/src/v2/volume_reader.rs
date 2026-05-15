// SPDX-License-Identifier: GPL-3.0-or-later
//
// 26.3 Cross-Platform Volume Reader — v2 stub.
//
// Faz 1: yalnızca Windows. MFT direkt okuma (ntfs crate) + FindFirstFile
// fallback. Bölüm 5'in tamamı Win32 bağımlı.
//
// Faz 2: Linux ext4/btrfs/zfs ve macOS APFS desteği. Her platform için
// VolumeBackend impl. Aynı `ScanTree` veri yapısı dönmeli.

use crate::error::Result;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub enum VolumeBackend {
    /// Bölüm 5 Windows MFT.
    WindowsNtfs,
    /// FindFirstFile / readdir generic.
    GenericReadDir,
    /// Linux ext4/btrfs/zfs (v2).
    LinuxFs,
    /// macOS APFS (v2).
    AppleApfs,
}

/// Bölüm 26.3 — cross-platform reader. Her impl `RawMftEntry` benzeri
/// bir akış üretmeli ki `build_tree` doğrudan tüketsin.
pub trait CrossPlatformVolumeReader: Send + Sync {
    fn backend(&self) -> VolumeBackend;

    /// Volume root path'i (Windows `C:\`, Linux `/mnt/...`, macOS `/Volumes/...`).
    /// Stream-tipli — büyük volume'larda memory patlamasını engeller.
    fn enumerate_entries(&self, root: &str) -> Result<Box<dyn Iterator<Item = String>>>;
}
