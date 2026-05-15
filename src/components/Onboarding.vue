<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<!--
  Onboarding — Bölüm 15.3 (İlk Açılış Deneyimi) + Bölüm 37.

  Beş adımlı akış:
    1) 3-slide tanıtım (atlanabilir) — D-Space ne yapar
    2) Hızlı / Standart mod seçimi (Bölüm 5.2A)
    3+) Otomatik tarama + sonuç tooltip — App.vue içinde kontrol edilir
  v0.1: ilk iki adım. Bölüm 37 competitive benchmark v0.2 (canlı rakip
  süreleri için Bölüm 37.2 etik sınırlar çerçevesinde).

  Settings'te `onboarding_done = "1"` flag — bir kez gösterilir.
-->
<script setup lang="ts">
import { ref, computed } from "vue";
import { useI18n } from "vue-i18n";

const { t } = useI18n();

type Mode = "fast" | "standard";

interface Slide {
  emoji: string;
  titleKey: string;
  bodyKey: string;
}

const SLIDES: Slide[] = [
  { emoji: "📊", titleKey: "onboarding.slide1Title", bodyKey: "onboarding.slide1Body" },
  { emoji: "🧠", titleKey: "onboarding.slide2Title", bodyKey: "onboarding.slide2Body" },
  { emoji: "↩", titleKey: "onboarding.slide3Title", bodyKey: "onboarding.slide3Body" },
];

defineProps<{ visible: boolean }>();
const emit = defineEmits<{ (e: "finish", mode: Mode): void }>();

const step = ref<"slides" | "mode">("slides");
const slideIndex = ref<number>(0);
const selectedMode = ref<Mode | null>(null);

const currentSlide = computed(() => SLIDES[slideIndex.value]);
const isLastSlide = computed(() => slideIndex.value === SLIDES.length - 1);

function next() {
  if (slideIndex.value < SLIDES.length - 1) {
    slideIndex.value++;
  } else {
    step.value = "mode";
  }
}

function back() {
  if (slideIndex.value > 0) {
    slideIndex.value--;
  }
}

function skipSlides() {
  step.value = "mode";
}

function selectMode(m: Mode) {
  selectedMode.value = m;
}

function finish() {
  if (!selectedMode.value) return;
  emit("finish", selectedMode.value);
}
</script>

<template>
  <div v-if="visible" class="onboard-backdrop">
    <div class="onboard-card">
      <header class="onboard-head">
        <span class="logo-dot"></span>
        <h2>{{ t("app.name") }}</h2>
        <span class="tagline mono">{{ t("onboarding.tagline") }}</span>
      </header>

      <template v-if="step === 'slides'">
        <div class="slide">
          <div class="slide-emoji">{{ currentSlide.emoji }}</div>
          <h3 class="slide-title">{{ t(currentSlide.titleKey) }}</h3>
          <p class="slide-body">{{ t(currentSlide.bodyKey) }}</p>
        </div>

        <div class="slide-dots">
          <span
            v-for="(_, i) in SLIDES"
            :key="i"
            class="dot"
            :class="{ 'dot-active': i === slideIndex }"
          ></span>
        </div>

        <div class="onboard-actions">
          <button
            type="button"
            class="onboard-btn onboard-ghost"
            :disabled="slideIndex === 0"
            @click="back"
          >
            {{ t("onboarding.back") }}
          </button>
          <button
            type="button"
            class="onboard-btn onboard-ghost"
            @click="skipSlides"
          >
            {{ t("onboarding.skip") }}
          </button>
          <button type="button" class="onboard-btn onboard-primary" @click="next">
            {{ isLastSlide ? t("onboarding.continue") : t("onboarding.next") }}
          </button>
        </div>
      </template>

      <template v-else>
        <h3 class="mode-title">{{ t("onboarding.modeTitle") }}</h3>
        <p class="mode-help">{{ t("onboarding.modeHelp") }}</p>

        <div class="mode-grid">
          <button
            type="button"
            class="mode-card"
            :class="{ 'mode-card-active': selectedMode === 'fast' }"
            @click="selectMode('fast')"
          >
            <div class="mode-icon">⚡</div>
            <div class="mode-name">{{ t("onboarding.modeFastName") }}</div>
            <div class="mode-pitch">{{ t("onboarding.modeFastPitch") }}</div>
            <ul class="mode-bullets">
              <li v-html="t('onboarding.modeFastB1')"></li>
              <li>{{ t("onboarding.modeFastB2") }}</li>
              <li>{{ t("onboarding.modeFastB3") }}</li>
            </ul>
          </button>

          <button
            type="button"
            class="mode-card"
            :class="{ 'mode-card-active': selectedMode === 'standard' }"
            @click="selectMode('standard')"
          >
            <div class="mode-icon">🛡</div>
            <div class="mode-name">{{ t("onboarding.modeStandardName") }}</div>
            <div class="mode-pitch">{{ t("onboarding.modeStandardPitch") }}</div>
            <ul class="mode-bullets">
              <li v-html="t('onboarding.modeStandardB1')"></li>
              <li>{{ t("onboarding.modeStandardB2") }}</li>
              <li>{{ t("onboarding.modeStandardB3") }}</li>
            </ul>
          </button>
        </div>

        <div class="onboard-actions">
          <button
            type="button"
            class="onboard-btn onboard-ghost"
            @click="step = 'slides'"
          >
            {{ t("onboarding.back") }}
          </button>
          <button
            type="button"
            class="onboard-btn onboard-primary"
            :disabled="!selectedMode"
            @click="finish"
          >
            {{ t("onboarding.start") }}
          </button>
        </div>
      </template>

      <footer class="onboard-foot mono">
        {{ t("onboarding.license") }}
      </footer>
    </div>
  </div>
</template>

<style scoped>
.onboard-backdrop {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.7);
  backdrop-filter: blur(6px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 200;
}

.onboard-card {
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 16px;
  padding: 28px 32px;
  max-width: 640px;
  width: calc(100% - 32px);
  box-shadow: 0 24px 64px rgba(0, 0, 0, 0.6);
  display: flex;
  flex-direction: column;
  gap: 20px;
}

.onboard-head {
  display: flex;
  align-items: center;
  gap: 12px;
}

.logo-dot {
  width: 14px;
  height: 14px;
  border-radius: 50%;
  background: radial-gradient(circle at 30% 30%, #24c8db, #0a6a78);
  box-shadow: 0 0 16px #24c8db66;
}

.onboard-head h2 {
  margin: 0;
  font-size: 22px;
  font-weight: 600;
}

.tagline {
  margin-left: auto;
  color: var(--muted);
  font-size: 11px;
  letter-spacing: 0.04em;
}

.slide {
  text-align: center;
  padding: 24px 8px 12px;
}

.slide-emoji {
  font-size: 64px;
  line-height: 1;
  margin-bottom: 14px;
}

.slide-title {
  margin: 0 0 12px;
  font-size: 22px;
  font-weight: 600;
  color: var(--fg);
}

.slide-body {
  margin: 0;
  color: var(--muted);
  font-size: 14px;
  line-height: 1.6;
  max-width: 420px;
  margin-inline: auto;
}

.slide-dots {
  display: flex;
  justify-content: center;
  gap: 6px;
}

.dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--border);
  transition: background 0.2s;
}

.dot-active {
  background: #24c8db;
  box-shadow: 0 0 8px #24c8db88;
}

.mode-title {
  margin: 0;
  font-size: 16px;
  font-weight: 600;
  color: var(--fg);
}

.mode-help {
  margin: 0;
  color: var(--muted);
  font-size: 12px;
  line-height: 1.5;
}

.mode-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 12px;
}

.mode-card {
  background: var(--bg);
  border: 1px solid var(--border);
  border-radius: 12px;
  padding: 16px;
  text-align: left;
  cursor: pointer;
  color: var(--fg);
  display: flex;
  flex-direction: column;
  gap: 8px;
  transition: background 0.15s, border-color 0.15s, transform 0.15s;
}

.mode-card:hover {
  background: #1f6f7c22;
  border-color: #2a8a99;
}

.mode-card-active {
  background: #1f6f7c33;
  border-color: #24c8db;
  transform: translateY(-2px);
}

.mode-icon {
  font-size: 28px;
}

.mode-name {
  font-size: 14px;
  font-weight: 600;
}

.mode-pitch {
  font-size: 12px;
  color: var(--muted);
}

.mode-bullets {
  margin: 4px 0 0;
  padding-left: 18px;
  font-size: 11px;
  color: var(--muted);
  display: flex;
  flex-direction: column;
  gap: 3px;
}

.mode-bullets strong {
  color: var(--fg);
}

.onboard-actions {
  display: flex;
  gap: 8px;
  justify-content: flex-end;
}

.onboard-btn {
  padding: 8px 16px;
  border-radius: 8px;
  font-size: 13px;
  cursor: pointer;
  border: 1px solid var(--border);
  background: var(--bg);
  color: var(--fg);
  transition: background 0.15s, border-color 0.15s;
}

.onboard-btn:hover:not(:disabled) {
  background: #1f6f7c22;
  border-color: #2a8a99;
}

.onboard-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.onboard-ghost {
  color: var(--muted);
}

.onboard-primary {
  background: #1f6f7c;
  border-color: #2a8a99;
  color: #e7fafe;
  font-weight: 500;
}

.onboard-primary:hover:not(:disabled) {
  background: #2a8a99;
}

.onboard-foot {
  font-size: 10px;
  color: var(--muted);
  text-align: center;
  letter-spacing: 0.04em;
  padding-top: 8px;
  border-top: 1px dashed var(--border);
}
</style>
