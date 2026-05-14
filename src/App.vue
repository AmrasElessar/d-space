<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
import { ref, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";

interface AppInfo {
  name: string;
  version: string;
  spec_version: string;
  spec_status: string;
  license: string;
  platform: string;
}

type ScanStrategy =
  | "MftService"
  | "DirectRawVolume"
  | "FindFirstFileFallback";

interface PrivilegeStatus {
  elevated: boolean;
  strategy: ScanStrategy;
}

interface MftProbe {
  drive: string;
  volume_path: string;
  volume_serial: number;
  cluster_size: number;
  sector_size: number;
  file_record_size: number;
  elapsed_ms: number;
}

type VolumeStatusKind =
  | "Ready"
  | "BitLockerLocked"
  | "BitLockerSuspended"
  | "Encrypted"
  | "NotMounted"
  | "AccessDenied"
  | "NotFormatted"
  | "UnsupportedDriveType";

interface VolumeStatus {
  kind: VolumeStatusKind;
  data?: { method?: string; drive_type?: number };
}

type DriveKind =
  | "Unknown"
  | "NoRootDir"
  | "Removable"
  | "Fixed"
  | "Remote"
  | "CdRom"
  | "RamDisk";

interface VolumeInfo {
  drive_letter: string;
  root_path: string;
  file_system: string;
  volume_label: string;
  volume_serial: number;
  drive_kind: DriveKind;
  total_bytes: number;
  free_bytes: number;
  status: VolumeStatus;
  elapsed_ms: number;
}

interface DspaceError {
  kind: string;
  message: string;
}

const info = ref<AppInfo | null>(null);
const ipcError = ref<string | null>(null);

const privilege = ref<PrivilegeStatus | null>(null);
const drive = ref<string>("C");
const probe = ref<MftProbe | null>(null);
const probeError = ref<string | null>(null);
const probing = ref(false);

const volumeInfo = ref<VolumeInfo | null>(null);
const volumeError = ref<string | null>(null);
const preFlighting = ref(false);

function formatHex(n: number): string {
  return "0x" + n.toString(16).toUpperCase().padStart(8, "0");
}

function formatBytes(b: number): string {
  if (b <= 0) return "—";
  const units = ["B", "KB", "MB", "GB", "TB"];
  let v = b;
  let i = 0;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i++;
  }
  return `${v.toFixed(i >= 3 ? 2 : 0)} ${units[i]}`;
}

function volumeStatusLabel(s: VolumeStatus): string {
  switch (s.kind) {
    case "Ready":
      return "Hazır";
    case "BitLockerLocked":
      return "BitLocker kilitli";
    case "BitLockerSuspended":
      return "BitLocker askıda";
    case "Encrypted":
      return `Şifreli (${s.data?.method ?? "bilinmeyen"})`;
    case "NotMounted":
      return "Bağlı değil";
    case "AccessDenied":
      return "Erişim engellendi";
    case "NotFormatted":
      return "Formatsız";
    case "UnsupportedDriveType":
      return `Desteklenmeyen sürücü (${s.data?.drive_type ?? "?"})`;
  }
}

function statusPillClass(s: VolumeStatus): string {
  return s.kind === "Ready" ? "pill-ok" : "pill-warn";
}

function strategyLabel(s: ScanStrategy): string {
  switch (s) {
    case "MftService":
      return "MFT Service (Katman 3)";
    case "DirectRawVolume":
      return "Hızlı Mod (Katman 1)";
    case "FindFirstFileFallback":
      return "Standart Mod (Katman 2)";
  }
}

onMounted(async () => {
  try {
    info.value = await invoke<AppInfo>("get_app_info");
    privilege.value = await invoke<PrivilegeStatus>("check_privilege");
  } catch (err) {
    ipcError.value = String(err);
  }
});

function formatIpcError(err: unknown): string {
  if (typeof err === "string") return err;
  const e = err as DspaceError;
  if (e && e.kind && e.message) return `[${e.kind}] ${e.message}`;
  return JSON.stringify(err);
}

async function runPreFlight() {
  preFlighting.value = true;
  volumeError.value = null;
  volumeInfo.value = null;
  try {
    volumeInfo.value = await invoke<VolumeInfo>("pre_flight_volume", {
      drive: drive.value,
    });
  } catch (err) {
    volumeError.value = formatIpcError(err);
  } finally {
    preFlighting.value = false;
  }
}

async function runProbe() {
  probing.value = true;
  probeError.value = null;
  probe.value = null;
  try {
    probe.value = await invoke<MftProbe>("probe_volume", {
      drive: drive.value,
    });
  } catch (err) {
    probeError.value = formatIpcError(err);
  } finally {
    probing.value = false;
  }
}
</script>

<template>
  <main class="shell">
    <header class="hero">
      <div class="brand">
        <span class="logo-dot"></span>
        <h1>{{ info?.name ?? "D-Space" }}</h1>
      </div>
      <p class="tagline">Görmek, anlamak, geri kazanmak.</p>
    </header>

    <section class="card">
      <h2>Durum</h2>
      <div v-if="info" class="grid">
        <div class="row">
          <span class="key">Sürüm</span>
          <span class="val mono">v{{ info.version }}</span>
        </div>
        <div class="row">
          <span class="key">Spec</span>
          <span class="val mono">{{ info.spec_version }}</span>
          <span class="pill pill-frozen">{{ info.spec_status }}</span>
        </div>
        <div class="row">
          <span class="key">Lisans</span>
          <span class="val mono">{{ info.license }}</span>
        </div>
        <div class="row">
          <span class="key">Platform</span>
          <span class="val mono">{{ info.platform }}</span>
        </div>
      </div>
      <p v-else-if="ipcError" class="err">IPC hatası: {{ ipcError }}</p>
      <p v-else class="muted">Sürüm bilgisi alınıyor…</p>
    </section>

    <section class="card">
      <h2>Yetki Durumu</h2>
      <div v-if="privilege" class="grid">
        <div class="row">
          <span class="key">Elevation</span>
          <span class="val mono">{{ privilege.elevated ? "evet" : "hayır" }}</span>
          <span class="pill" :class="privilege.elevated ? 'pill-ok' : 'pill-warn'">
            {{ privilege.elevated ? "admin" : "standart" }}
          </span>
        </div>
        <div class="row">
          <span class="key">Strateji</span>
          <span class="val">{{ strategyLabel(privilege.strategy) }}</span>
        </div>
      </div>
    </section>

    <section class="card">
      <h2>Volume Pre-Flight (Bölüm 33.2 Katman 0)</h2>
      <div class="probe-bar">
        <input
          v-model="drive"
          maxlength="3"
          spellcheck="false"
          class="drive-input mono"
          aria-label="Sürücü harfi"
        />
        <button
          type="button"
          class="probe-btn"
          :disabled="preFlighting"
          @click="runPreFlight"
        >
          {{ preFlighting ? "Sorgulanıyor…" : "Pre-flight" }}
        </button>
      </div>
      <div v-if="volumeInfo" class="grid">
        <div class="row">
          <span class="key">Durum</span>
          <span class="val">{{ volumeStatusLabel(volumeInfo.status) }}</span>
          <span class="pill" :class="statusPillClass(volumeInfo.status)">
            {{ volumeInfo.status.kind }}
          </span>
        </div>
        <div class="row">
          <span class="key">Sürücü</span>
          <span class="val mono">{{ volumeInfo.root_path }}</span>
        </div>
        <div class="row">
          <span class="key">FS</span>
          <span class="val mono">{{ volumeInfo.file_system || "—" }}</span>
        </div>
        <div class="row">
          <span class="key">Etiket</span>
          <span class="val">{{ volumeInfo.volume_label || "—" }}</span>
        </div>
        <div class="row">
          <span class="key">Serial</span>
          <span class="val mono">{{ formatHex(volumeInfo.volume_serial) }}</span>
        </div>
        <div class="row">
          <span class="key">Tip</span>
          <span class="val">{{ volumeInfo.drive_kind }}</span>
        </div>
        <div class="row">
          <span class="key">Toplam</span>
          <span class="val mono">{{ formatBytes(volumeInfo.total_bytes) }}</span>
        </div>
        <div class="row">
          <span class="key">Boş</span>
          <span class="val mono">{{ formatBytes(volumeInfo.free_bytes) }}</span>
        </div>
        <div class="row">
          <span class="key">Süre</span>
          <span class="val mono">{{ volumeInfo.elapsed_ms }} ms</span>
        </div>
      </div>
      <p v-if="volumeError" class="err">{{ volumeError }}</p>
    </section>

    <section class="card">
      <h2>MFT Probe (Bölüm 5)</h2>
      <div class="probe-bar">
        <input
          v-model="drive"
          maxlength="3"
          spellcheck="false"
          class="drive-input mono"
          aria-label="Sürücü harfi"
        />
        <button
          type="button"
          class="probe-btn"
          :disabled="probing"
          @click="runProbe"
        >
          {{ probing ? "Probe çalışıyor…" : "Probe çalıştır" }}
        </button>
      </div>
      <div v-if="probe" class="grid">
        <div class="row">
          <span class="key">Volume</span>
          <span class="val mono">{{ probe.volume_path }}</span>
        </div>
        <div class="row">
          <span class="key">Serial</span>
          <span class="val mono">{{ formatHex(probe.volume_serial) }}</span>
        </div>
        <div class="row">
          <span class="key">Cluster</span>
          <span class="val mono">{{ probe.cluster_size }} B</span>
        </div>
        <div class="row">
          <span class="key">Sector</span>
          <span class="val mono">{{ probe.sector_size }} B</span>
        </div>
        <div class="row">
          <span class="key">MFT rec.</span>
          <span class="val mono">{{ probe.file_record_size }} B</span>
        </div>
        <div class="row">
          <span class="key">Süre</span>
          <span class="val mono">{{ probe.elapsed_ms }} ms</span>
        </div>
      </div>
      <p v-if="probeError" class="err">{{ probeError }}</p>
    </section>

    <section class="card">
      <h2>Yol Haritası</h2>
      <ol class="roadmap">
        <li class="done">Spec v1.4 donduruldu (37 bölüm, 4 edge case kapatıldı)</li>
        <li class="active">Faz 1 implementasyon başlangıcı — proje iskeleti</li>
        <li>MFT tarama motoru (Bölüm 5)</li>
        <li>Staging + Undo + WAL (Bölüm 12)</li>
        <li>Sunburst + treemap görselleştirme (Bölüm 9)</li>
        <li>Safe-to-delete kural motoru (Bölüm 6)</li>
        <li>Snapshot / Time Machine (Bölüm 8)</li>
      </ol>
    </section>

    <footer class="foot">
      <span>GPL-3.0-or-later</span>
      <span class="dot">·</span>
      <span>Master mimari: D-Space-Mimari-v1.4.docx</span>
    </footer>
  </main>
</template>

<style scoped>
.shell {
  max-width: 880px;
  margin: 0 auto;
  padding: 48px 32px 32px;
  display: flex;
  flex-direction: column;
  gap: 24px;
}

.hero {
  text-align: left;
}

.brand {
  display: flex;
  align-items: center;
  gap: 14px;
}

.logo-dot {
  width: 14px;
  height: 14px;
  border-radius: 50%;
  background: radial-gradient(circle at 30% 30%, #24c8db, #0a6a78);
  box-shadow: 0 0 16px #24c8db66;
}

.hero h1 {
  margin: 0;
  font-size: 32px;
  letter-spacing: -0.02em;
  font-weight: 600;
}

.tagline {
  margin: 8px 0 0;
  color: var(--muted);
  font-size: 15px;
}

.card {
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 14px;
  padding: 20px 22px;
}

.card h2 {
  margin: 0 0 14px;
  font-size: 14px;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  color: var(--muted);
  font-weight: 600;
}

.grid {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.row {
  display: flex;
  align-items: center;
  gap: 12px;
  font-size: 14px;
}

.key {
  width: 90px;
  color: var(--muted);
}

.val {
  color: var(--fg);
}

.mono {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 13px;
}

.pill {
  font-size: 11px;
  padding: 2px 8px;
  border-radius: 999px;
  letter-spacing: 0.04em;
  font-weight: 600;
}

.pill-frozen {
  background: #1e3a8a33;
  color: #93c5fd;
  border: 1px solid #1e3a8a66;
}

.pill-ok {
  background: #14532d33;
  color: #6ee7b7;
  border: 1px solid #14532d66;
}

.pill-warn {
  background: #78350f33;
  color: #fcd34d;
  border: 1px solid #78350f66;
}

.probe-bar {
  display: flex;
  gap: 8px;
  margin-bottom: 14px;
}

.drive-input {
  width: 64px;
  padding: 8px 10px;
  background: var(--bg);
  border: 1px solid var(--border);
  border-radius: 8px;
  color: var(--fg);
  text-align: center;
  text-transform: uppercase;
}

.drive-input:focus {
  outline: none;
  border-color: #24c8db;
}

.probe-btn {
  padding: 8px 16px;
  background: #1f6f7c;
  border: 1px solid #2a8a99;
  border-radius: 8px;
  color: #e7fafe;
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  transition: background 0.15s, border-color 0.15s;
}

.probe-btn:hover:not(:disabled) {
  background: #2a8a99;
}

.probe-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.roadmap {
  margin: 0;
  padding-left: 18px;
  display: flex;
  flex-direction: column;
  gap: 6px;
  font-size: 14px;
  color: var(--muted);
}

.roadmap li.done {
  color: #6ee7b7;
  text-decoration: line-through;
  text-decoration-color: #6ee7b766;
}

.roadmap li.active {
  color: var(--fg);
  font-weight: 600;
}

.err {
  color: #fca5a5;
  font-family: ui-monospace, monospace;
  font-size: 13px;
}

.muted {
  color: var(--muted);
  font-size: 14px;
}

.foot {
  display: flex;
  gap: 8px;
  font-size: 12px;
  color: var(--muted);
  padding-top: 8px;
}

.dot {
  opacity: 0.5;
}
</style>

<style>
:root {
  --fg: #e5e7eb;
  --muted: #9ca3af;
  --bg: #0b0d10;
  --surface: #14171c;
  --border: #1f242c;

  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Inter, sans-serif;
  font-size: 14px;
  line-height: 1.5;
  color: var(--fg);
  background: var(--bg);
  font-synthesis: none;
  text-rendering: optimizeLegibility;
  -webkit-font-smoothing: antialiased;
}

* {
  box-sizing: border-box;
}

html, body, #app {
  margin: 0;
  padding: 0;
  min-height: 100vh;
}
</style>
