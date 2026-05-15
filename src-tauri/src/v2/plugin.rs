// SPDX-License-Identifier: GPL-3.0-or-later
//
// 26.4 Plugin Sistemi — v2 stub (kapalı kutu).
//
// Faz 2'de kullanıcı tanımlı kural setleri + view mode + safe-delete
// scorer eklentileri. Sandbox: WASM (wasmtime) veya restricted Rust
// dynamic library — kararı v2 başında ver.
//
// Plugin yetenekleri kısıtlı: dosya sistemine doğrudan erişim YOK,
// yalnızca read-only metadata + UI extension point'ler.

use crate::error::Result;
use serde::Serialize;

#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct PluginCapabilities {
    pub adds_rules: bool,
    pub adds_view_mode: bool,
    pub adds_scorer: bool,
    pub reads_telemetry: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct PluginContext {
    pub plugin_id: String,
    pub version: String,
    pub capabilities: PluginCapabilities,
}

/// Bölüm 26.4 — plugin trait. v2'de gerçek implementation; v1.x stub.
pub trait Plugin: Send + Sync {
    fn context(&self) -> &PluginContext;

    /// İlk yükleme. Manifest doğrulama, capability kabul.
    fn on_load(&mut self) -> Result<()>;

    /// Boşaltma. Cleanup yapsın.
    fn on_unload(&mut self) -> Result<()>;
}
