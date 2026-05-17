<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<!--
  Bölüm başlıklarının yanındaki küçük ⓘ butonu — tıklayınca açılan popover
  başlığın altına anlatım metni gösterir. Toggle: ikinci tıklama veya
  dışarı tıklama ile kapanır.

  Kullanım:
    <h2>
      Başlık <InfoButton :text="t('section.intro')" />
    </h2>
-->
<script setup lang="ts">
import { ref, onBeforeUnmount, watchEffect } from "vue";

defineProps<{ text: string; ariaLabel?: string }>();

const open = ref(false);
const rootEl = ref<HTMLElement | null>(null);

function toggle(): void {
  open.value = !open.value;
}

function close(): void {
  open.value = false;
}

function onDocumentClick(e: MouseEvent): void {
  if (!rootEl.value) return;
  if (e.target instanceof Node && rootEl.value.contains(e.target)) return;
  close();
}

function onKey(e: KeyboardEvent): void {
  if (e.key === "Escape") close();
}

watchEffect((onCleanup) => {
  if (open.value) {
    document.addEventListener("click", onDocumentClick, true);
    document.addEventListener("keydown", onKey, true);
    onCleanup(() => {
      document.removeEventListener("click", onDocumentClick, true);
      document.removeEventListener("keydown", onKey, true);
    });
  }
});

onBeforeUnmount(() => {
  document.removeEventListener("click", onDocumentClick, true);
  document.removeEventListener("keydown", onKey, true);
});
</script>

<template>
  <span class="info-wrap" ref="rootEl">
    <button
      type="button"
      class="info-btn"
      :class="{ 'info-btn-active': open }"
      :aria-label="ariaLabel ?? 'Bilgi'"
      :aria-expanded="open"
      @click.stop="toggle"
    >
      ⓘ
    </button>
    <transition name="info-pop">
      <span v-if="open" class="info-pop" role="tooltip">{{ text }}</span>
    </transition>
  </span>
</template>

<style scoped>
.info-wrap {
  position: relative;
  display: inline-flex;
  align-items: center;
  vertical-align: middle;
  margin-left: 6px;
}

.info-btn {
  width: 18px;
  height: 18px;
  border: 1px solid var(--border);
  background: transparent;
  color: var(--muted);
  border-radius: 999px;
  font-size: 11px;
  line-height: 1;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  padding: 0;
  transition: color 0.15s, border-color 0.15s, background 0.15s, transform 0.1s;
}

.info-btn:hover {
  color: var(--fg);
  border-color: var(--fg);
}

.info-btn:active {
  transform: scale(0.92);
}

.info-btn-active {
  color: var(--fg);
  background: var(--bg);
  border-color: var(--fg);
}

.info-pop {
  position: absolute;
  top: calc(100% + 8px);
  left: 0;
  z-index: 50;
  width: max-content;
  max-width: 360px;
  padding: 10px 12px;
  background: var(--surface);
  color: var(--fg);
  border: 1px solid var(--border);
  border-radius: 8px;
  box-shadow: 0 8px 20px var(--shadow);
  font-size: 12px;
  font-weight: 400;
  text-transform: none;
  letter-spacing: normal;
  line-height: 1.55;
  white-space: normal;
}

.info-pop::before {
  content: "";
  position: absolute;
  top: -5px;
  left: 8px;
  width: 8px;
  height: 8px;
  background: var(--surface);
  border-left: 1px solid var(--border);
  border-top: 1px solid var(--border);
  transform: rotate(45deg);
}

.info-pop-enter-from,
.info-pop-leave-to {
  opacity: 0;
  transform: translateY(-4px);
}

.info-pop-enter-active,
.info-pop-leave-active {
  transition: opacity 0.12s ease, transform 0.12s ease;
}
</style>
