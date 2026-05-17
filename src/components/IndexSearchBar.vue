<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<!--
  Hızlı Arama (Quick Search) çubuğu — Sprint 3.8 / Discovery Log #005
  (Bölüm 5.6). NTFS USN Journal index'i üzerinden Everything benzeri
  anlık substring araması.

  Davranış:
    * Sticky üst konum (top: 0, position: sticky). App.vue'ya yerleştirilir.
    * Input change → 150 ms debounce → `index_search(query, limit=50)` IPC.
    * Boş query → sonuç listesi kapanır.
    * Her sonuç tıklanınca `update:selected-result` emit edilir
      (App.vue ana panel detayına yönlendirebilir).
    * `index_status` ile mode = "needs_admin" gelirse input devre dışı,
      yardım metni gösterilir.

  v0.1 sınırları:
    * Yalnız isim substring eşleşmesi (LIKE '%query%'); fuzzy/score yok.
    * Maks 50 sonuç. UI ileri/geri scroll yok.
    * full_path opsiyonel — backend WITH RECURSIVE çözer; UI tooltip'te
      gösterir, satır ana metni isim.
-->
<script setup lang="ts">
import { computed, onMounted, ref, watch, onBeforeUnmount } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useI18n } from "vue-i18n";

const { t } = useI18n();

interface IndexSearchResult {
  volume_id: string;
  file_ref: number;
  parent_ref: number;
  name: string;
  full_path: string | null;
  attrs: number;
}

interface IndexStatus {
  volume_id: string | null;
  total_entries: number;
  last_sync_unix: number;
  mode: "ready" | "building" | "idle" | "needs_admin" | "error" | string;
}

interface DspaceError {
  kind: string;
  message: string;
}

const query = ref<string>("");
const results = ref<IndexSearchResult[]>([]);
const busy = ref<boolean>(false);
const error = ref<string | null>(null);
const status = ref<IndexStatus | null>(null);

const DEBOUNCE_MS = 150;
const LIMIT = 50;
let debounceTimer: ReturnType<typeof setTimeout> | null = null;

const emit = defineEmits<{
  (e: "update:selected-result", payload: IndexSearchResult): void;
}>();

const isAdminBlocked = computed(() => status.value?.mode === "needs_admin");
const placeholder = computed(() =>
  isAdminBlocked.value ? t("index.needsAdmin") : t("index.placeholder"),
);

function formatIpcError(err: unknown): string {
  if (typeof err === "string") return err;
  const e = err as DspaceError;
  if (e && e.kind && e.message) return `[${e.kind}] ${e.message}`;
  return JSON.stringify(err);
}

async function refreshStatus() {
  try {
    status.value = await invoke<IndexStatus>("index_status", { drive: null });
  } catch (err) {
    error.value = formatIpcError(err);
  }
}

async function runSearch(q: string) {
  if (!q.trim()) {
    results.value = [];
    busy.value = false;
    return;
  }
  busy.value = true;
  error.value = null;
  try {
    results.value = await invoke<IndexSearchResult[]>("index_search", {
      query: q,
      limit: LIMIT,
    });
  } catch (err) {
    error.value = formatIpcError(err);
    results.value = [];
  } finally {
    busy.value = false;
  }
}

watch(query, (newQ) => {
  if (debounceTimer) {
    clearTimeout(debounceTimer);
    debounceTimer = null;
  }
  if (!newQ.trim()) {
    results.value = [];
    busy.value = false;
    return;
  }
  busy.value = true;
  debounceTimer = setTimeout(() => {
    void runSearch(newQ);
  }, DEBOUNCE_MS);
});

function selectResult(r: IndexSearchResult) {
  emit("update:selected-result", r);
}

onMounted(() => {
  void refreshStatus();
});

onBeforeUnmount(() => {
  if (debounceTimer) {
    clearTimeout(debounceTimer);
    debounceTimer = null;
  }
});
</script>

<template>
  <div class="index-search-bar" data-testid="index-search-bar">
    <label class="search-row">
      <span class="search-icon" aria-hidden="true">🔎</span>
      <input
        type="text"
        v-model="query"
        :placeholder="placeholder"
        :disabled="isAdminBlocked"
        :aria-label="t('index.title')"
        autocomplete="off"
        spellcheck="false"
        data-testid="index-search-input"
      />
      <span v-if="busy" class="status-pill" data-testid="index-search-busy">
        {{ t("index.searching") }}
      </span>
      <span
        v-else-if="status && status.total_entries > 0"
        class="status-pill subtle"
      >
        {{ status.total_entries }}
      </span>
    </label>

    <div v-if="error" class="search-error" role="alert">{{ error }}</div>

    <ul
      v-if="query.trim() && !busy && results.length === 0"
      class="result-list empty"
      data-testid="index-search-empty"
    >
      <li class="muted">{{ t("index.empty") }}</li>
    </ul>

    <ul
      v-else-if="results.length > 0"
      class="result-list"
      data-testid="index-search-results"
    >
      <li
        v-for="r in results"
        :key="`${r.volume_id}:${r.file_ref}`"
        class="result-row"
        :title="r.full_path ?? r.name"
        tabindex="0"
        @click="selectResult(r)"
        @keydown.enter="selectResult(r)"
      >
        <span class="result-icon" aria-hidden="true">
          {{ (r.attrs & 0x10) !== 0 ? "📁" : "📄" }}
        </span>
        <span class="result-name">{{ r.name }}</span>
        <span v-if="r.full_path" class="result-path">{{ r.full_path }}</span>
      </li>
    </ul>
  </div>
</template>

<style scoped>
.index-search-bar {
  position: sticky;
  top: 0;
  z-index: 20;
  background: var(--bg);
  border-bottom: 1px solid var(--border);
  padding: 8px 12px;
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Inter, sans-serif;
}

.search-row {
  display: flex;
  align-items: center;
  gap: 8px;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 4px 10px;
  transition: border-color 0.15s, box-shadow 0.15s;
}

.search-row:focus-within {
  border-color: #4f8cff;
  box-shadow: 0 0 0 2px rgba(79, 140, 255, 0.18);
}

.search-icon {
  font-size: 14px;
  opacity: 0.7;
}

.search-row input {
  flex: 1;
  background: transparent;
  border: none;
  outline: none;
  color: var(--fg);
  font-size: 14px;
  padding: 6px 0;
  min-width: 0;
}

.search-row input:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.status-pill {
  font-size: 11px;
  padding: 2px 8px;
  border-radius: 999px;
  background: rgba(79, 140, 255, 0.16);
  color: #4f8cff;
  border: 1px solid rgba(79, 140, 255, 0.3);
  white-space: nowrap;
}

.status-pill.subtle {
  background: var(--bg);
  color: var(--muted);
  border-color: var(--border);
}

.search-error {
  margin-top: 6px;
  font-size: 12px;
  color: var(--err, #ef4444);
}

.result-list {
  list-style: none;
  margin: 6px 0 0 0;
  padding: 0;
  max-height: 380px;
  overflow-y: auto;
  border: 1px solid var(--border);
  border-radius: 6px;
  background: var(--surface);
  box-shadow: 0 4px 12px var(--shadow);
}

.result-list.empty li {
  padding: 8px 12px;
  font-style: italic;
}

.result-row {
  display: grid;
  grid-template-columns: 22px 1fr auto;
  align-items: center;
  gap: 8px;
  padding: 6px 10px;
  cursor: pointer;
  border-bottom: 1px solid var(--border);
  font-size: 13px;
  transition: background 0.1s;
}

.result-row:last-child {
  border-bottom: none;
}

.result-row:hover,
.result-row:focus {
  background: var(--bg);
  outline: none;
}

.result-icon {
  opacity: 0.85;
}

.result-name {
  color: var(--fg);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.result-path {
  color: var(--muted);
  font-size: 11px;
  font-family: ui-monospace, "Cascadia Code", "Consolas", monospace;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 240px;
}

.muted {
  color: var(--muted);
}
</style>
