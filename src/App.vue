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

const info = ref<AppInfo | null>(null);
const ipcError = ref<string | null>(null);

onMounted(async () => {
  try {
    info.value = await invoke<AppInfo>("get_app_info");
  } catch (err) {
    ipcError.value = String(err);
  }
});
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
