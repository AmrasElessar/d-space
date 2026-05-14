<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
import { ref, onMounted, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import Sunburst from "./components/Sunburst.vue";
import SnapshotPanel from "./components/SnapshotPanel.vue";

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

interface DbInfo {
  path: string;
  schema_version: number;
  journal_mode: string;
  page_size: number;
  table_count: number;
  spec_version: string;
}

interface StagedItem {
  id: number;
  original_path: string;
  staged_path: string;
  size_bytes: number;
  staged_at_unix: number;
  expires_at_unix: number;
  is_dir: boolean;
  fallback_tier: string;
}

interface MftWalkStats {
  drive: string;
  volume_path: string;
  total_records_estimate: number;
  records_visited: number;
  in_use_records: number;
  directory_records: number;
  file_records: number;
  skipped_errors: number;
  bytes_aggregate: number;
  sample_names: string[];
  elapsed_ms: number;
}

interface ScanSummary {
  drive: string;
  volume_id: string;
  strategy: ScanStrategy;
  root_id: number;
  node_count: number;
  file_count: number;
  dir_count: number;
  total_bytes: number;
  collect_elapsed_ms: number;
  build_elapsed_ms: number;
}

interface TreeNode {
  id: number;
  parent: number | null;
  name: string;
  data_size: number;
  aggregate_size: number;
  is_dir: boolean;
  score: number | null;
  score_rule: string | null;
  score_reason: string | null;
}

type SortKey = "size_desc" | "name_asc" | "data_size_desc";

interface WindowResult {
  parent_id: number;
  parent_name: string;
  parent_aggregate_size: number;
  total_children: number;
  returned: number;
  nodes: TreeNode[];
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

const dbInfo = ref<DbInfo | null>(null);
const dbError = ref<string | null>(null);

const walkStats = ref<MftWalkStats | null>(null);
const walkError = ref<string | null>(null);
const walking = ref(false);

const scanSummary = ref<ScanSummary | null>(null);
const scanError = ref<string | null>(null);
const scanning = ref(false);

const viewWindow = ref<WindowResult | null>(null);
const breadcrumb = ref<TreeNode[]>([]);
const sortKey = ref<SortKey>("size_desc");
const windowError = ref<string | null>(null);
const windowLoading = ref(false);

const stagingList = ref<StagedItem[]>([]);
const stagingError = ref<string | null>(null);
const stagingBusyId = ref<number | null>(null);
const stagePendingPath = ref<string | null>(null);

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
    ipcError.value = formatIpcError(err);
  }
  try {
    dbInfo.value = await invoke<DbInfo>("get_db_info");
  } catch (err) {
    dbError.value = formatIpcError(err);
  }
  await refreshStaging();
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

async function runWalk() {
  walking.value = true;
  walkError.value = null;
  walkStats.value = null;
  try {
    walkStats.value = await invoke<MftWalkStats>("walk_volume", {
      drive: drive.value,
    });
  } catch (err) {
    walkError.value = formatIpcError(err);
  } finally {
    walking.value = false;
  }
}

async function loadWindow(parentId: number) {
  windowLoading.value = true;
  windowError.value = null;
  try {
    const w = await invoke<WindowResult>("tree_window", {
      parent: parentId,
      sort: sortKey.value,
      limit: 200,
      offset: 0,
    });
    viewWindow.value = w;
    breadcrumb.value = await invoke<TreeNode[]>("tree_path", { id: parentId });
  } catch (err) {
    windowError.value = formatIpcError(err);
  } finally {
    windowLoading.value = false;
  }
}

async function runFullScan() {
  scanning.value = true;
  scanError.value = null;
  scanSummary.value = null;
  viewWindow.value = null;
  breadcrumb.value = [];
  try {
    scanSummary.value = await invoke<ScanSummary>("scan_full", {
      drive: drive.value,
    });
    await loadWindow(scanSummary.value.root_id);
  } catch (err) {
    scanError.value = formatIpcError(err);
  } finally {
    scanning.value = false;
  }
}

function drillInto(node: TreeNode) {
  if (node.is_dir) {
    loadWindow(node.id);
  }
}

function goUp(id: number) {
  loadWindow(id);
}

watch(sortKey, () => {
  if (viewWindow.value) {
    loadWindow(viewWindow.value.parent_id);
  }
});

function percentOf(part: number, whole: number): string {
  if (whole <= 0) return "—";
  const pct = (part / whole) * 100;
  if (pct < 0.1) return "<0.1%";
  return `${pct.toFixed(1)}%`;
}

function scoreTierClass(score: number | null): string {
  if (score === null) return "";
  if (score <= 30) return "score-danger";
  if (score <= 60) return "score-caution";
  if (score <= 85) return "score-likely";
  return "score-cache";
}

function driveLetter(): string {
  const m = drive.value.toUpperCase().match(/[A-Z]/);
  return m ? m[0] : "C";
}

function nodeFullPath(n: TreeNode): string {
  const letter = driveLetter();
  const parts = breadcrumb.value.slice(1).map((b) => b.name);
  parts.push(n.name);
  return `${letter}:\\${parts.join("\\")}`;
}

function formatTime(unix: number): string {
  if (!unix) return "—";
  const d = new Date(unix * 1000);
  return d.toLocaleString("tr-TR");
}

async function refreshStaging() {
  try {
    stagingList.value = await invoke<StagedItem[]>("list_staging");
  } catch (err) {
    stagingError.value = formatIpcError(err);
  }
}

async function confirmStage(node: TreeNode) {
  const path = nodeFullPath(node);
  if (stagePendingPath.value === path) {
    // Confirmed
    stagePendingPath.value = null;
    stagingBusyId.value = node.id;
    stagingError.value = null;
    try {
      await invoke<StagedItem>("stage_path", { path });
      await refreshStaging();
      if (viewWindow.value) {
        await loadWindow(viewWindow.value.parent_id);
      }
    } catch (err) {
      stagingError.value = formatIpcError(err);
    } finally {
      stagingBusyId.value = null;
    }
  } else {
    stagePendingPath.value = path;
  }
}

function cancelStage() {
  stagePendingPath.value = null;
}

async function runUndo(id: number) {
  stagingBusyId.value = id;
  stagingError.value = null;
  try {
    await invoke<string>("undo_staging", { id });
    await refreshStaging();
    if (viewWindow.value) {
      await loadWindow(viewWindow.value.parent_id);
    }
  } catch (err) {
    stagingError.value = formatIpcError(err);
  } finally {
    stagingBusyId.value = null;
  }
}

function scoreTierLabel(score: number | null): string {
  if (score === null) return "—";
  if (score <= 30) return "DOKUNMA";
  if (score <= 60) return "İNCELE";
  if (score <= 85) return "BÜYÜK İHTİMAL";
  return "CACHE";
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
      <h2>MFT Full Walk (Bölüm 5.1 + 4.3 Adım 2)</h2>
      <div class="probe-bar">
        <button
          type="button"
          class="probe-btn"
          :disabled="walking"
          @click="runWalk"
        >
          {{ walking ? `MFT taranıyor… (${drive})` : `MFT walk: ${drive}` }}
        </button>
      </div>
      <div v-if="walkStats" class="grid">
        <div class="row">
          <span class="key">Tahmin</span>
          <span class="val mono">
            {{ walkStats.total_records_estimate.toLocaleString("tr-TR") }} record
          </span>
        </div>
        <div class="row">
          <span class="key">Gezildi</span>
          <span class="val mono">
            {{ walkStats.records_visited.toLocaleString("tr-TR") }}
          </span>
          <span class="pill pill-ok">
            {{ walkStats.in_use_records.toLocaleString("tr-TR") }} in-use
          </span>
        </div>
        <div class="row">
          <span class="key">Klasör</span>
          <span class="val mono">
            {{ walkStats.directory_records.toLocaleString("tr-TR") }}
          </span>
        </div>
        <div class="row">
          <span class="key">Dosya</span>
          <span class="val mono">
            {{ walkStats.file_records.toLocaleString("tr-TR") }}
          </span>
        </div>
        <div class="row">
          <span class="key">Toplam</span>
          <span class="val mono">{{ formatBytes(walkStats.bytes_aggregate) }}</span>
        </div>
        <div class="row">
          <span class="key">Atlanan</span>
          <span class="val mono">{{ walkStats.skipped_errors }}</span>
        </div>
        <div class="row">
          <span class="key">Süre</span>
          <span class="val mono">{{ walkStats.elapsed_ms }} ms</span>
        </div>
        <div v-if="walkStats.sample_names.length" class="samples">
          <div class="samples-title">Örnek isimler ({{ walkStats.sample_names.length }})</div>
          <ul class="samples-list mono">
            <li v-for="(n, i) in walkStats.sample_names" :key="i">{{ n }}</li>
          </ul>
        </div>
      </div>
      <p v-if="walkError" class="err">{{ walkError }}</p>
    </section>

    <section class="card">
      <h2>Tam Tarama + ScanTree (Bölüm 4.3 + 4.4)</h2>
      <div class="probe-bar">
        <button
          type="button"
          class="probe-btn"
          :disabled="scanning"
          @click="runFullScan"
        >
          {{ scanning ? `Taranıyor ${drive}…` : `Tam tarama: ${drive}` }}
        </button>
      </div>
      <div v-if="scanSummary" class="grid">
        <div class="row">
          <span class="key">Strateji</span>
          <span class="val">{{ strategyLabel(scanSummary.strategy) }}</span>
          <span
            class="pill"
            :class="
              scanSummary.strategy === 'DirectRawVolume'
                ? 'pill-ok'
                : 'pill-warn'
            "
          >
            {{ scanSummary.strategy === "DirectRawVolume" ? "MFT" : "FALLBACK" }}
          </span>
        </div>
        <div class="row">
          <span class="key">Düğüm</span>
          <span class="val mono">
            {{ scanSummary.node_count.toLocaleString("tr-TR") }}
          </span>
        </div>
        <div class="row">
          <span class="key">Dosya</span>
          <span class="val mono">
            {{ scanSummary.file_count.toLocaleString("tr-TR") }}
          </span>
        </div>
        <div class="row">
          <span class="key">Klasör</span>
          <span class="val mono">
            {{ scanSummary.dir_count.toLocaleString("tr-TR") }}
          </span>
        </div>
        <div class="row">
          <span class="key">Toplam</span>
          <span class="val mono">{{ formatBytes(scanSummary.total_bytes) }}</span>
        </div>
        <div class="row">
          <span class="key">Toplama</span>
          <span class="val mono">{{ scanSummary.collect_elapsed_ms }} ms</span>
        </div>
        <div class="row">
          <span class="key">Ağaç build</span>
          <span class="val mono">{{ scanSummary.build_elapsed_ms }} ms</span>
        </div>
      </div>
      <p v-if="scanError" class="err">{{ scanError }}</p>
    </section>

    <section v-if="viewWindow" class="card">
      <h2>Drilldown (Bölüm 9.6 viewport query)</h2>

      <Sunburst :data="viewWindow" @drill="drillInto" />

      <nav class="crumbs">
        <template v-for="(c, i) in breadcrumb" :key="c.id">
          <button
            type="button"
            class="crumb"
            :class="{ 'crumb-current': i === breadcrumb.length - 1 }"
            :disabled="i === breadcrumb.length - 1"
            @click="goUp(c.id)"
          >
            {{ c.name }}
          </button>
          <span v-if="i < breadcrumb.length - 1" class="crumb-sep">›</span>
        </template>
      </nav>

      <div class="drill-bar">
        <label class="sort-label">
          Sırala:
          <select v-model="sortKey" class="sort-select">
            <option value="size_desc">Boyut ↓</option>
            <option value="name_asc">İsim ↑</option>
            <option value="data_size_desc">Veri boyutu ↓</option>
          </select>
        </label>
        <span class="drill-stats mono">
          {{ viewWindow.returned }} / {{ viewWindow.total_children }} ·
          {{ formatBytes(viewWindow.parent_aggregate_size) }}
        </span>
      </div>

      <ul class="drill-list">
        <li
          v-for="n in viewWindow.nodes"
          :key="n.id"
          class="drill-row"
          :class="{ 'drill-dir': n.is_dir, 'drill-file': !n.is_dir }"
          @click="drillInto(n)"
        >
          <span class="drill-icon">{{ n.is_dir ? "📁" : "📄" }}</span>
          <span class="drill-name mono">{{ n.name }}</span>
          <span
            v-if="n.score !== null"
            class="score-pill"
            :class="scoreTierClass(n.score)"
            :title="n.score_reason ?? ''"
          >
            {{ n.score }} · {{ scoreTierLabel(n.score) }}
          </span>
          <span v-else class="score-pill score-none">—</span>
          <span class="drill-bar-inner">
            <span
              class="drill-fill"
              :style="{
                width: percentOf(n.aggregate_size, viewWindow.parent_aggregate_size),
              }"
            ></span>
          </span>
          <span class="drill-pct mono">
            {{ percentOf(n.aggregate_size, viewWindow.parent_aggregate_size) }}
          </span>
          <span class="drill-size mono">
            {{ formatBytes(n.aggregate_size) }}
          </span>
          <span class="drill-actions" @click.stop>
            <template v-if="stagePendingPath === nodeFullPath(n)">
              <button
                type="button"
                class="stage-btn stage-confirm"
                :disabled="stagingBusyId === n.id"
                @click="confirmStage(n)"
              >
                ✓ Onayla
              </button>
              <button
                type="button"
                class="stage-btn stage-cancel"
                @click="cancelStage"
              >
                ✕
              </button>
            </template>
            <button
              v-else
              type="button"
              class="stage-btn"
              :disabled="stagingBusyId === n.id"
              title="Staging'e gönder (24h içinde geri alınabilir)"
              @click="confirmStage(n)"
            >
              📥
            </button>
          </span>
        </li>
        <li v-if="windowLoading" class="drill-loading">Yükleniyor…</li>
      </ul>
      <p v-if="windowError" class="err">{{ windowError }}</p>
    </section>

    <section class="card">
      <h2>Staging Kuyruğu (Bölüm 12)</h2>
      <p class="muted" v-if="stagingList.length === 0 && !stagingError">
        Henüz staging'e atılmış öğe yok. Drilldown'da 📥 butonu ile gönderebilirsin —
        24 saat içinde ↩ ile geri alınabilir.
      </p>
      <ul v-if="stagingList.length" class="staging-list">
        <li v-for="item in stagingList" :key="item.id" class="staging-row">
          <span class="staging-icon">{{ item.is_dir ? "📁" : "📄" }}</span>
          <span class="staging-path mono">{{ item.original_path }}</span>
          <span
            class="tier-pill"
            :class="
              item.fallback_tier === 'cross_volume'
                ? 'tier-cross'
                : 'tier-normal'
            "
            :title="
              item.fallback_tier === 'cross_volume'
                ? 'Cross-volume two-phase commit (Bölüm 12.3): Blake3 hash verify + WAL + atomik rename'
                : 'Same-volume atomik rename (Bölüm 12.2)'
            "
          >
            {{ item.fallback_tier === "cross_volume" ? "2PC" : "MOVE" }}
          </span>
          <span class="staging-size mono">{{ formatBytes(item.size_bytes) }}</span>
          <span class="staging-time mono">{{ formatTime(item.staged_at_unix) }}</span>
          <button
            type="button"
            class="stage-btn"
            :disabled="stagingBusyId === item.id"
            title="Orijinal yoluna geri taşı"
            @click="runUndo(item.id)"
          >
            ↩ Undo
          </button>
        </li>
      </ul>
      <p v-if="stagingError" class="err">{{ stagingError }}</p>
    </section>

    <SnapshotPanel />

    <section class="card">
      <h2>Veritabanı (Bölüm 14)</h2>
      <div v-if="dbInfo" class="grid">
        <div class="row">
          <span class="key">Dosya</span>
          <span class="val mono path">{{ dbInfo.path }}</span>
        </div>
        <div class="row">
          <span class="key">Schema</span>
          <span class="val mono">v{{ dbInfo.schema_version }}</span>
          <span class="pill pill-ok">{{ dbInfo.table_count }} tablo</span>
        </div>
        <div class="row">
          <span class="key">Journal</span>
          <span class="val mono">{{ dbInfo.journal_mode.toUpperCase() }}</span>
        </div>
        <div class="row">
          <span class="key">Page</span>
          <span class="val mono">{{ dbInfo.page_size }} B</span>
        </div>
        <div class="row">
          <span class="key">Spec</span>
          <span class="val mono">{{ dbInfo.spec_version || "—" }}</span>
        </div>
      </div>
      <p v-if="dbError" class="err">{{ dbError }}</p>
    </section>

    <section class="card">
      <h2>Yol Haritası</h2>
      <ol class="roadmap">
        <li class="done">Spec v1.4 donduruldu (37 bölüm, 4 edge case kapatıldı)</li>
        <li class="done">Proje iskeleti — Tauri 2 + Vue 3 + GPL-3.0-or-later</li>
        <li class="done">MFT probe + yetki kontrolü (Bölüm 5)</li>
        <li class="done">Volume pre-flight (Bölüm 33.2 Katman 0)</li>
        <li class="done">SQLite + migrations 0001 (Bölüm 14)</li>
        <li class="done">MFT full walk v0.1 (Bölüm 5.1 + 4.3 Adım 2)</li>
        <li class="done">ScanTree builder + agregat boyutlar (Bölüm 4.3+4.4)</li>
        <li class="done">FindFirstFile fallback + auto-strategy (Bölüm 5.2A K2)</li>
        <li class="done">Lazy viewport query + drilldown (Bölüm 9.6)</li>
        <li class="done">Sunburst donut (Bölüm 9.1 Pillar 2) — SVG hand-rolled</li>
        <li class="done">Safe-to-delete kural motoru — 33 kural (Bölüm 6)</li>
        <li class="done">Staging + Undo same-volume (Bölüm 12.2)</li>
        <li class="done">Cross-volume two-phase commit + WAL (Bölüm 12.3)</li>
        <li class="active">Time Machine / Snapshot (Bölüm 8)</li>
        <li>Duplicate Detector (Bölüm 7, Blake3 hazır)</li>
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

.path {
  font-size: 11px;
  word-break: break-all;
  color: var(--muted);
}

.samples {
  margin-top: 6px;
  padding-top: 12px;
  border-top: 1px dashed var(--border);
}

.samples-title {
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: var(--muted);
  margin-bottom: 6px;
}

.samples-list {
  margin: 0;
  padding-left: 16px;
  font-size: 12px;
  max-height: 200px;
  overflow-y: auto;
  list-style: square;
}

.samples-list li {
  color: var(--fg);
  padding: 1px 0;
}

.top-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
  max-height: 320px;
  overflow-y: auto;
}

.top-item {
  display: grid;
  grid-template-columns: 20px 1fr 100px;
  gap: 10px;
  align-items: center;
  padding: 4px 8px;
  border-radius: 6px;
  font-size: 13px;
}

.top-item:hover {
  background: var(--bg);
}

.top-icon {
  font-size: 14px;
  text-align: center;
}

.top-name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--fg);
}

.top-size {
  text-align: right;
  color: #6ee7b7;
  font-weight: 500;
}

.crumbs {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 4px;
  margin-bottom: 12px;
  font-size: 13px;
}

.crumb {
  background: transparent;
  border: none;
  color: #93c5fd;
  cursor: pointer;
  padding: 4px 8px;
  border-radius: 6px;
  font-size: 13px;
  font-family: ui-monospace, monospace;
  transition: background 0.15s;
}

.crumb:hover:not(:disabled) {
  background: #1e3a8a33;
}

.crumb-current {
  color: var(--fg);
  cursor: default;
}

.crumb-sep {
  color: var(--muted);
  font-size: 12px;
}

.drill-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 12px;
  padding-bottom: 10px;
  border-bottom: 1px solid var(--border);
}

.sort-label {
  font-size: 12px;
  color: var(--muted);
  display: flex;
  align-items: center;
  gap: 8px;
}

.sort-select {
  background: var(--bg);
  border: 1px solid var(--border);
  border-radius: 6px;
  color: var(--fg);
  padding: 4px 8px;
  font-size: 12px;
  font-family: inherit;
  cursor: pointer;
}

.drill-stats {
  font-size: 11px;
  color: var(--muted);
}

.drill-list {
  list-style: none;
  margin: 0;
  padding: 0;
  max-height: 500px;
  overflow-y: auto;
}

.drill-row {
  display: grid;
  grid-template-columns: 22px 1fr 130px 100px 60px 100px 130px;
  align-items: center;
  gap: 10px;
  padding: 6px 8px;
  font-size: 13px;
  border-radius: 6px;
  border: 1px solid transparent;
}

.drill-actions {
  display: flex;
  gap: 4px;
  justify-content: flex-end;
}

.stage-btn {
  background: transparent;
  border: 1px solid var(--border);
  color: var(--fg);
  padding: 3px 8px;
  border-radius: 6px;
  font-size: 13px;
  cursor: pointer;
  transition: background 0.15s, border-color 0.15s;
}

.stage-btn:hover:not(:disabled) {
  background: #1f6f7c33;
  border-color: #24c8db66;
}

.stage-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.stage-confirm {
  background: #14532d66;
  border-color: #14532d;
  color: #6ee7b7;
  font-weight: 600;
  font-size: 11px;
}

.stage-confirm:hover:not(:disabled) {
  background: #14532daa;
}

.stage-cancel {
  background: transparent;
  border-color: var(--border);
  color: var(--muted);
  font-size: 11px;
  padding: 3px 6px;
}

.staging-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 4px;
  max-height: 360px;
  overflow-y: auto;
}

.staging-row {
  display: grid;
  grid-template-columns: 22px 1fr 50px 100px 150px 100px;
  gap: 10px;
  align-items: center;
  padding: 6px 8px;
  font-size: 12px;
  border-radius: 6px;
  border: 1px solid var(--border);
  background: var(--bg);
}

.tier-pill {
  font-size: 10px;
  font-weight: 700;
  padding: 2px 6px;
  border-radius: 4px;
  letter-spacing: 0.04em;
  text-align: center;
  font-family: ui-monospace, monospace;
}

.tier-normal {
  background: #14171c;
  color: #6ee7b7;
  border: 1px solid #14532d66;
}

.tier-cross {
  background: #1e3a8a33;
  color: #93c5fd;
  border: 1px solid #1e3a8a;
}

.staging-icon {
  text-align: center;
}

.staging-path {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--fg);
  font-size: 11px;
}

.staging-size {
  text-align: right;
  color: #6ee7b7;
}

.staging-time {
  color: var(--muted);
  font-size: 11px;
  text-align: right;
}

.score-pill {
  font-size: 10px;
  font-weight: 600;
  padding: 2px 8px;
  border-radius: 999px;
  text-align: center;
  letter-spacing: 0.04em;
  white-space: nowrap;
  font-family: ui-monospace, monospace;
}

.score-danger {
  background: #7f1d1d33;
  color: #fca5a5;
  border: 1px solid #7f1d1d80;
}

.score-caution {
  background: #78350f33;
  color: #fcd34d;
  border: 1px solid #78350f80;
}

.score-likely {
  background: #14532d33;
  color: #6ee7b7;
  border: 1px solid #14532d80;
}

.score-cache {
  background: #1e3a8a33;
  color: #93c5fd;
  border: 1px solid #1e3a8a80;
}

.score-none {
  background: transparent;
  color: var(--muted);
  border: 1px dashed var(--border);
}

.drill-dir {
  cursor: pointer;
}

.drill-dir:hover {
  background: var(--bg);
  border-color: var(--border);
}

.drill-file {
  cursor: default;
  opacity: 0.85;
}

.drill-icon {
  text-align: center;
}

.drill-name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.drill-bar-inner {
  position: relative;
  height: 8px;
  background: var(--bg);
  border-radius: 4px;
  overflow: hidden;
}

.drill-fill {
  position: absolute;
  top: 0;
  left: 0;
  height: 100%;
  background: linear-gradient(90deg, #24c8db, #1e6f7c);
  border-radius: 4px;
}

.drill-pct {
  text-align: right;
  color: var(--muted);
  font-size: 11px;
}

.drill-size {
  text-align: right;
  color: #6ee7b7;
  font-weight: 500;
}

.drill-loading {
  padding: 12px;
  text-align: center;
  color: var(--muted);
  font-size: 12px;
  list-style: none;
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
