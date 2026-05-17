<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<!--
  Tarama Progress overlay — Bölüm 9.6.5 + 15.4.

  Backend `scan-progress` event'lerini dinler. Tam ekran overlay:
  faz adı, sayım, son taranan path, hız (file/sn), tahmini ETA.
  Sprint 3.7 — split layout: sol canlı sunburst (LiveSunburst), sağ
  sayısal stats. `partial_tree` her 10k entry'de backend'den taşınır.
  Tarama bitince fade-out.
-->
<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import LiveSunburst from "./LiveSunburst.vue";

interface PartialNode {
  id: number;
  parent: number | null;
  name: string;
  aggregate_size: number;
  depth: number;
  is_dir: boolean;
}

interface ScanProgressEvent {
  phase: string;
  visited: number;
  total_estimate: number;
  in_use: number;
  last_name: string;
  elapsed_ms: number;
  partial_tree?: PartialNode[] | null;
}

const { t, locale } = useI18n();

const props = defineProps<{
  visible: boolean;
  progress: ScanProgressEvent | null;
}>();

// Sprint 3.7 — backend yalnızca 10k entry'de partial_tree gönderir, ara
// event'ler `null/undefined`. Son alınan dolu tree'yi belleğe alıp
// LiveSunburst'a stabil veri akışı sağlarız. visible=false olunca
// (overlay kapanınca) bayatlamamak için sıfırlanır.
const latestPartialTree = ref<PartialNode[]>([]);

// Canlı log — son 12 taranan path'i tut (en yeni başta). Tekrar eden
// last_name'i göstermeye gerek yok (backend her event'te aynı path'i de
// gönderiyor olabilir, dedupe).
interface LogEntry {
  key: number;
  path: string;
}
const recentLog = ref<LogEntry[]>([]);
const MAX_LOG = 12;
let lastSeenPath = "";
let logSeq = 0;

watch(
  () => props.progress?.partial_tree,
  (incoming) => {
    if (incoming && incoming.length > 0) {
      latestPartialTree.value = incoming;
    }
  },
);

watch(
  () => props.progress?.last_name,
  (name) => {
    if (!name || name === lastSeenPath) return;
    lastSeenPath = name;
    logSeq += 1;
    recentLog.value = [
      { key: logSeq, path: name },
      ...recentLog.value.slice(0, MAX_LOG - 1),
    ];
  },
);

watch(
  () => props.visible,
  (v) => {
    if (!v) {
      latestPartialTree.value = [];
      recentLog.value = [];
      lastSeenPath = "";
    }
  },
);

const phaseLabel = computed(() => {
  if (!props.progress) return t("scanPhase.preparing");
  switch (props.progress.phase) {
    case "mft_walk":
      return t("scanPhase.mft");
    case "fallback_walk":
      return t("scanPhase.fallback");
    case "done":
      return t("scanPhase.done");
    default:
      return props.progress.phase;
  }
});

const percent = computed(() => {
  if (!props.progress) return 0;
  const p = props.progress;
  if (p.total_estimate <= 0) return 0;
  return Math.min(99, Math.floor((p.visited / p.total_estimate) * 100));
});

const speedPerSec = computed(() => {
  if (!props.progress || props.progress.elapsed_ms <= 0) return 0;
  return Math.floor(
    (props.progress.visited * 1000) / props.progress.elapsed_ms,
  );
});

const etaSec = computed(() => {
  if (!props.progress) return null;
  const p = props.progress;
  if (p.total_estimate <= 0 || speedPerSec.value <= 0) return null;
  const remaining = Math.max(0, p.total_estimate - p.visited);
  return Math.ceil(remaining / speedPerSec.value);
});

function localeTag(): string {
  return locale.value === "tr" ? "tr-TR" : "en-US";
}

function formatNumber(n: number): string {
  return n.toLocaleString(localeTag());
}

function formatMs(ms: number): string {
  const unit = locale.value === "tr" ? "sn" : "s";
  if (ms < 1000) return `${ms} ms`;
  return `${(ms / 1000).toFixed(1)} ${unit}`;
}

function formatEta(sec: number | null): string {
  if (sec === null) return "—";
  const unit = locale.value === "tr" ? "sn" : "s";
  const mLabel = locale.value === "tr" ? "dk" : "min";
  if (sec < 60) return `~${sec} ${unit}`;
  const m = Math.floor(sec / 60);
  const s = sec % 60;
  return `~${m}:${s.toString().padStart(2, "0")} ${mLabel}`;
}

function truncatePath(p: string, max = 60): string {
  if (p.length <= max) return p;
  return "…" + p.slice(-max + 1);
}
</script>

<template>
  <div v-if="visible" class="scan-overlay">
    <div class="scan-panel">
      <div class="scan-header">
        <div class="spinner"></div>
        <h3 class="scan-phase">{{ phaseLabel }}</h3>
      </div>

      <div class="scan-body">
        <!-- Sol kolon: canlı sunburst (Sprint 3.7) -->
        <div class="scan-visual">
          <LiveSunburst
            :partial-tree="latestPartialTree"
            :empty-message="t('scanProgress.liveEmpty')"
          />
        </div>

        <!-- Sağ kolon: sayısal stats -->
        <div class="scan-info">
          <div class="scan-bar-wrap">
            <div class="scan-bar-track">
              <div
                class="scan-bar-fill"
                :style="{ width: percent > 0 ? `${percent}%` : '30%' }"
                :class="{ 'scan-bar-indeterminate': percent === 0 }"
              ></div>
            </div>
            <span v-if="percent > 0" class="scan-percent mono">{{ percent }}%</span>
          </div>

          <div v-if="progress" class="scan-stats">
            <div class="stat">
              <span class="stat-key">{{ t("scanStats.scanned") }}</span>
              <span class="stat-val mono">{{ formatNumber(progress.visited) }}</span>
            </div>
            <div v-if="progress.total_estimate > 0" class="stat">
              <span class="stat-key">{{ t("scanStats.estimated") }}</span>
              <span class="stat-val mono">
                {{ formatNumber(progress.total_estimate) }}
              </span>
            </div>
            <div class="stat">
              <span class="stat-key">{{ t("scanStats.speed") }}</span>
              <span class="stat-val mono">
                {{ formatNumber(speedPerSec)
                }}{{ locale === "tr" ? "/sn" : "/s" }}
              </span>
            </div>
            <div class="stat">
              <span class="stat-key">{{ t("scanStats.elapsed") }}</span>
              <span class="stat-val mono">{{ formatMs(progress.elapsed_ms) }}</span>
            </div>
            <div class="stat">
              <span class="stat-key">{{ t("scanStats.remaining") }}</span>
              <span class="stat-val mono">{{ formatEta(etaSec) }}</span>
            </div>
          </div>

          <div class="scan-log">
            <div class="scan-log-head">
              <span class="dot-live"></span>
              <span class="scan-log-title">{{ t("scanProgress.logTitle") }}</span>
            </div>
            <transition-group name="log-row" tag="ul" class="scan-log-list">
              <li
                v-for="entry in recentLog"
                :key="entry.key"
                class="scan-log-row mono"
              >
                {{ truncatePath(entry.path, 64) }}
              </li>
              <li
                v-if="recentLog.length === 0"
                key="empty"
                class="scan-log-row muted"
              >
                {{ t("scanProgress.scanning") }}
              </li>
            </transition-group>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.scan-overlay {
  position: fixed;
  inset: 0;
  background: color-mix(in srgb, var(--bg) 92%, transparent);
  backdrop-filter: blur(8px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 150;
}

.scan-panel {
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 16px;
  padding: 28px 36px;
  max-width: 820px;
  width: calc(100% - 32px);
  box-shadow: 0 24px 64px var(--shadow);
  display: flex;
  flex-direction: column;
  gap: 18px;
}

.scan-body {
  display: flex;
  gap: 28px;
  align-items: stretch;
}

.scan-visual {
  flex: 0 0 360px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.scan-info {
  flex: 1 1 auto;
  display: flex;
  flex-direction: column;
  gap: 14px;
  min-width: 0;
}

@media (max-width: 720px) {
  .scan-body {
    flex-direction: column;
  }
  .scan-visual {
    flex: 0 0 auto;
  }
}

.scan-header {
  display: flex;
  align-items: center;
  gap: 14px;
}

.spinner {
  width: 22px;
  height: 22px;
  border: 3px solid var(--border);
  border-top-color: #24c8db;
  border-radius: 50%;
  animation: spin 0.9s linear infinite;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

.scan-phase {
  margin: 0;
  font-size: 16px;
  font-weight: 600;
  color: var(--fg);
}

.scan-bar-wrap {
  display: flex;
  align-items: center;
  gap: 12px;
}

.scan-bar-track {
  flex: 1;
  height: 8px;
  background: var(--bg);
  border-radius: 4px;
  overflow: hidden;
  position: relative;
}

.scan-bar-fill {
  height: 100%;
  background: linear-gradient(90deg, #24c8db, #1e6f7c);
  border-radius: 4px;
  transition: width 0.3s ease-out;
}

.scan-bar-indeterminate {
  animation: indeterminate 1.4s ease-in-out infinite;
}

@keyframes indeterminate {
  0% {
    margin-left: -30%;
  }
  100% {
    margin-left: 100%;
  }
}

.scan-percent {
  font-size: 12px;
  color: var(--muted);
  min-width: 36px;
  text-align: right;
}

.scan-stats {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(110px, 1fr));
  gap: 10px;
}

.stat {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 8px 12px;
  background: var(--bg);
  border: 1px solid var(--border);
  border-radius: 8px;
}

.stat-key {
  font-size: 10px;
  color: var(--muted);
  text-transform: uppercase;
  letter-spacing: 0.06em;
}

.stat-val {
  font-size: 14px;
  color: var(--fg);
  font-weight: 500;
}

.scan-log {
  display: flex;
  flex-direction: column;
  gap: 6px;
  background: var(--bg);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 8px 10px;
  max-height: 180px;
  overflow: hidden;
}

.scan-log-head {
  display: flex;
  align-items: center;
  gap: 6px;
}

.scan-log-title {
  font-size: 10px;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  color: var(--muted);
}

.dot-live {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: #24c8db;
  box-shadow: 0 0 0 0 rgba(36, 200, 219, 0.7);
  animation: dot-pulse 1.4s ease-in-out infinite;
}

@keyframes dot-pulse {
  0% {
    box-shadow: 0 0 0 0 rgba(36, 200, 219, 0.55);
  }
  70% {
    box-shadow: 0 0 0 8px rgba(36, 200, 219, 0);
  }
  100% {
    box-shadow: 0 0 0 0 rgba(36, 200, 219, 0);
  }
}

.scan-log-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
  overflow: hidden;
}

.scan-log-row {
  font-size: 11px;
  color: var(--fg);
  opacity: 0.85;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  transition: opacity 0.2s linear;
}

.scan-log-row:nth-child(1) {
  color: var(--fg);
  opacity: 1;
}
.scan-log-row:nth-child(n + 5) {
  opacity: 0.55;
}
.scan-log-row:nth-child(n + 9) {
  opacity: 0.35;
}

.log-row-enter-from {
  opacity: 0;
  transform: translateY(-6px);
}

.log-row-leave-to {
  opacity: 0;
  transform: translateY(4px);
}

.log-row-enter-active,
.log-row-leave-active {
  transition: opacity 0.18s ease, transform 0.18s ease;
}

.log-row-move {
  transition: transform 0.2s ease;
}

.muted {
  color: var(--muted);
  font-style: italic;
}
</style>
