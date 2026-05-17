<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<!--
  VolumeSidebar — Bölüm 15.1 v0.2 üç-kolon workspace'in sol kolonu.
  Win32 `GetLogicalDrives` üzerinden mount edilmiş sürücüleri listeler,
  her satır için pre-flight bilgisini (FS, kullanım, status) gösterir.
  Tıklanan sürücü `update:selected` event'i ile parent'a iletilir.
-->
<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useI18n } from "vue-i18n";

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

const props = defineProps<{
  selected: string;
}>();

const emit = defineEmits<{
  (e: "update:selected", drive: string): void;
}>();

const { t } = useI18n();

const drives = ref<VolumeInfo[]>([]);
const loading = ref<boolean>(false);
const error = ref<string | null>(null);

function formatIpcError(err: unknown): string {
  if (typeof err === "object" && err && "message" in err) {
    return (err as DspaceError).message;
  }
  return String(err);
}

function formatBytes(bytes: number): string {
  if (bytes <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB", "PB"];
  let val = bytes;
  let i = 0;
  while (val >= 1024 && i < units.length - 1) {
    val /= 1024;
    i++;
  }
  const precision = val >= 100 ? 0 : val >= 10 ? 1 : 2;
  return `${val.toFixed(precision)} ${units[i]}`;
}

/** Drive letter only (örn. "C") — props.selected ile karşılaştırma için. */
function letterOf(info: VolumeInfo): string {
  return info.drive_letter.replace(/[^A-Za-z]/g, "").toUpperCase();
}

function usagePercent(info: VolumeInfo): number {
  if (info.total_bytes <= 0) return 0;
  const used = info.total_bytes - info.free_bytes;
  return Math.max(0, Math.min(100, (used / info.total_bytes) * 100));
}

function usageClass(pct: number): string {
  if (pct >= 90) return "usage-danger";
  if (pct >= 75) return "usage-warn";
  return "usage-ok";
}

function statusLabel(kind: VolumeStatusKind): string {
  // i18n fallback'ten dolayı .raw alıyoruz: anahtar yoksa kind string'ini dön.
  const key = `sidebar.status.${kind}`;
  const out = t(key);
  return out === key ? kind : out;
}

function statusPillClass(kind: VolumeStatusKind): string {
  switch (kind) {
    case "Ready":
      return "status-ok";
    case "BitLockerLocked":
    case "BitLockerSuspended":
    case "Encrypted":
      return "status-locked";
    case "AccessDenied":
    case "NotFormatted":
    case "NotMounted":
      return "status-warn";
    default:
      return "status-warn";
  }
}

function kindLabel(kind: DriveKind): string {
  const key = `sidebar.kind.${kind}`;
  const out = t(key);
  return out === key ? kind : out;
}

function kindEmoji(kind: DriveKind): string {
  switch (kind) {
    case "Fixed":
      return "💾";
    case "Removable":
      return "💿";
    case "Remote":
      return "🌐";
    case "CdRom":
      return "📀";
    case "RamDisk":
      return "⚡";
    default:
      return "❓";
  }
}

const selectedLetter = computed(() => props.selected.toUpperCase().slice(0, 1));

async function refresh() {
  loading.value = true;
  error.value = null;
  try {
    drives.value = await invoke<VolumeInfo[]>("list_drives_cmd");
  } catch (err) {
    error.value = formatIpcError(err);
  } finally {
    loading.value = false;
  }
}

function selectDrive(info: VolumeInfo) {
  const letter = letterOf(info);
  if (!letter) return;
  emit("update:selected", letter);
}

function onKey(e: KeyboardEvent, info: VolumeInfo) {
  if (e.key === "Enter" || e.key === " ") {
    e.preventDefault();
    selectDrive(info);
  }
}

onMounted(refresh);

// Parent yeni bir sürücü seçtiyse listede yoksa fetch'i tetikle.
watch(
  () => props.selected,
  (next) => {
    if (!next) return;
    const letter = next.toUpperCase().slice(0, 1);
    if (drives.value.length === 0) return;
    if (!drives.value.some((d) => letterOf(d) === letter)) {
      void refresh();
    }
  },
);

defineExpose({ refresh });
</script>

<template>
  <aside class="sidebar" aria-label="Volume sidebar">
    <header class="sidebar-header">
      <h2 class="sidebar-title">{{ t("sidebar.title") }}</h2>
      <button
        type="button"
        class="refresh-btn"
        :disabled="loading"
        :title="t('sidebar.refresh')"
        @click="refresh"
      >
        {{ loading ? "↻" : "⟳" }}
      </button>
    </header>

    <p v-if="loading && drives.length === 0" class="sidebar-status">
      {{ t("sidebar.loading") }}
    </p>
    <p v-else-if="error" class="sidebar-err">
      {{ t("sidebar.error", { msg: error }) }}
    </p>
    <p v-else-if="drives.length === 0" class="sidebar-status">
      {{ t("sidebar.empty") }}
    </p>

    <ul v-else class="drive-list">
      <li
        v-for="info in drives"
        :key="info.root_path"
        class="drive-row"
        :class="{ 'drive-row-selected': letterOf(info) === selectedLetter }"
        tabindex="0"
        role="button"
        :aria-pressed="letterOf(info) === selectedLetter"
        @click="selectDrive(info)"
        @keydown="onKey($event, info)"
      >
        <div class="drive-head">
          <span class="drive-icon">{{ kindEmoji(info.drive_kind) }}</span>
          <span class="drive-letter mono">{{ info.drive_letter }}</span>
          <span class="drive-label">
            {{ info.volume_label || kindLabel(info.drive_kind) }}
          </span>
          <span class="status-pill" :class="statusPillClass(info.status.kind)">
            {{ statusLabel(info.status.kind) }}
          </span>
        </div>

        <div v-if="info.total_bytes > 0" class="drive-usage">
          <div class="usage-bar">
            <div
              class="usage-fill"
              :class="usageClass(usagePercent(info))"
              :style="{ width: usagePercent(info) + '%' }"
            />
          </div>
          <div class="usage-meta mono">
            {{
              t("sidebar.free", {
                free: formatBytes(info.free_bytes),
                total: formatBytes(info.total_bytes),
              })
            }}
          </div>
        </div>

        <div class="drive-foot">
          <span v-if="info.file_system" class="drive-fs mono">
            {{ info.file_system }}
          </span>
          <span class="drive-kind">{{ kindLabel(info.drive_kind) }}</span>
        </div>
      </li>
    </ul>
  </aside>
</template>

<style scoped>
.sidebar {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 14px 12px;
  border-radius: 10px;
  background: var(--surface);
  border: 1px solid var(--border);
  position: sticky;
  top: 16px;
  align-self: start;
  max-height: calc(100vh - 32px);
  overflow-y: auto;
}

.sidebar-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}

.sidebar-title {
  font-size: 12px;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  color: var(--muted);
  margin: 0;
}

.refresh-btn {
  background: transparent;
  border: 1px solid var(--border);
  color: var(--muted);
  width: 26px;
  height: 26px;
  border-radius: 6px;
  cursor: pointer;
  font-size: 13px;
  line-height: 1;
  transition: background 0.15s, color 0.15s, border-color 0.15s;
}

.refresh-btn:hover:not(:disabled) {
  background: var(--bg);
  color: var(--fg);
  border-color: #2a8a99;
}

.refresh-btn:disabled {
  opacity: 0.5;
  cursor: progress;
}

.sidebar-status {
  color: var(--muted);
  font-size: 12px;
  text-align: center;
  margin: 12px 0;
}

.sidebar-err {
  color: var(--err, #ef4444);
  font-size: 11px;
  margin: 8px 0;
  word-break: break-word;
}

.drive-list {
  list-style: none;
  padding: 0;
  margin: 0;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.drive-row {
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 10px;
  border-radius: 8px;
  border: 1px solid var(--border);
  background: var(--bg);
  cursor: pointer;
  transition: border-color 0.15s, background 0.15s, transform 0.05s;
  outline: none;
}

.drive-row:hover {
  border-color: #2a8a99;
}

.drive-row:focus-visible {
  border-color: #38bdf8;
  box-shadow: 0 0 0 2px #38bdf833;
}

.drive-row:active {
  transform: scale(0.99);
}

.drive-row-selected {
  border-color: #24c8db;
  background: #1f6f7c22;
  box-shadow: 0 0 0 1px #24c8db66 inset;
}

.drive-head {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-wrap: wrap;
}

.drive-icon {
  font-size: 14px;
}

.drive-letter {
  font-weight: 600;
  font-size: 14px;
  color: var(--fg);
}

.drive-label {
  flex: 1;
  font-size: 12px;
  color: var(--fg);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  min-width: 0;
}

.status-pill {
  font-size: 10px;
  padding: 2px 6px;
  border-radius: 999px;
  letter-spacing: 0.04em;
  border: 1px solid var(--border);
  background: var(--bg);
  white-space: nowrap;
}

.status-pill.status-ok {
  border-color: #16653466;
  background: #16653422;
  color: #86efac;
}

.status-pill.status-locked {
  border-color: #78350f66;
  background: #78350f22;
  color: #fcd34d;
}

.status-pill.status-warn {
  border-color: #7f1d1d66;
  background: #7f1d1d22;
  color: #fca5a5;
}

.drive-usage {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.usage-bar {
  width: 100%;
  height: 5px;
  border-radius: 3px;
  background: var(--border);
  overflow: hidden;
}

.usage-fill {
  height: 100%;
  border-radius: 3px;
  transition: width 0.25s ease;
}

.usage-fill.usage-ok {
  background: linear-gradient(90deg, #2dd4bf, #14b8a6);
}

.usage-fill.usage-warn {
  background: linear-gradient(90deg, #fbbf24, #f59e0b);
}

.usage-fill.usage-danger {
  background: linear-gradient(90deg, #f87171, #ef4444);
}

.usage-meta {
  font-size: 10px;
  color: var(--muted);
}

.drive-foot {
  display: flex;
  justify-content: space-between;
  gap: 6px;
  font-size: 10px;
  color: var(--muted);
}

.drive-fs {
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.mono {
  font-family: ui-monospace, "Cascadia Code", "Consolas", monospace;
}
</style>
