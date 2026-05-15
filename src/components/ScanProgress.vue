<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<!--
  Tarama Progress overlay — Bölüm 9.6.5 + 15.4.

  Backend `scan-progress` event'lerini dinler. Tam ekran overlay:
  faz adı, sayım, son taranan path, hız (file/sn), tahmini ETA.
  Tarama bitince fade-out.
-->
<script setup lang="ts">
import { computed } from "vue";

interface ScanProgressEvent {
  phase: string;
  visited: number;
  total_estimate: number;
  in_use: number;
  last_name: string;
  elapsed_ms: number;
}

const props = defineProps<{
  visible: boolean;
  progress: ScanProgressEvent | null;
}>();

const phaseLabel = computed(() => {
  if (!props.progress) return "Hazırlanıyor…";
  switch (props.progress.phase) {
    case "mft_walk":
      return "MFT okunuyor (Hızlı Mod)";
    case "fallback_walk":
      return "Klasörler taranıyor (Standart Mod)";
    case "done":
      return "Tarama tamamlandı";
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

function formatNumber(n: number): string {
  return n.toLocaleString("tr-TR");
}

function formatMs(ms: number): string {
  if (ms < 1000) return `${ms} ms`;
  return `${(ms / 1000).toFixed(1)} sn`;
}

function formatEta(sec: number | null): string {
  if (sec === null) return "—";
  if (sec < 60) return `~${sec} sn`;
  const m = Math.floor(sec / 60);
  const s = sec % 60;
  return `~${m}:${s.toString().padStart(2, "0")} dk`;
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
          <span class="stat-key">Tarandı</span>
          <span class="stat-val mono">{{ formatNumber(progress.visited) }}</span>
        </div>
        <div v-if="progress.total_estimate > 0" class="stat">
          <span class="stat-key">Tahmini</span>
          <span class="stat-val mono">
            {{ formatNumber(progress.total_estimate) }}
          </span>
        </div>
        <div class="stat">
          <span class="stat-key">Hız</span>
          <span class="stat-val mono">{{ formatNumber(speedPerSec) }}/sn</span>
        </div>
        <div class="stat">
          <span class="stat-key">Süre</span>
          <span class="stat-val mono">{{ formatMs(progress.elapsed_ms) }}</span>
        </div>
        <div class="stat">
          <span class="stat-key">Kalan</span>
          <span class="stat-val mono">{{ formatEta(etaSec) }}</span>
        </div>
      </div>

      <p v-if="progress && progress.last_name" class="scan-current mono">
        {{ truncatePath(progress.last_name) }}
      </p>
      <p v-else class="scan-current muted">Tarama başlatılıyor…</p>
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
  max-width: 560px;
  width: calc(100% - 32px);
  box-shadow: 0 24px 64px var(--shadow);
  display: flex;
  flex-direction: column;
  gap: 18px;
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

.scan-current {
  margin: 0;
  font-size: 11px;
  color: var(--muted);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  background: var(--bg);
  border: 1px dashed var(--border);
  border-radius: 6px;
  padding: 6px 10px;
}

.muted {
  color: var(--muted);
  font-style: italic;
}
</style>
