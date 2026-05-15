<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<!--
  Bubble pack — Bölüm 9.1 mod 3/4 (dört görüntü modu), Pillar 2.

  Force-relaxation circle packing:
    1. Daireleri alan-bazlı yarıçapla hesapla (r = √(area/π)).
    2. Spiral seed: en büyüğü merkeze, sonrakiler Fibonacci açıyla dış spirale.
    3. 80 iter overlap-push + center-pull → çakışmasız stabil layout.
    4. Sonuç deterministic (seed yok), tek seferlik computed.

  Hand-rolled, D3 yok. N=200 max düğüm için ~50ms compute.
  Renk Bölüm 6.3 skor tier'ından; klasörler tıklanır → drill emit.
-->
<script setup lang="ts">
import { computed, ref } from "vue";

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
  modified_unix: number;
}

interface WindowResult {
  parent_id: number;
  parent_name: string;
  parent_aggregate_size: number;
  total_children: number;
  returned: number;
  nodes: TreeNode[];
}

interface Bubble {
  id: number;
  name: string;
  size: number;
  is_dir: boolean;
  x: number;
  y: number;
  r: number;
  fill: string;
  scoreLabel: string;
}

const props = defineProps<{ data: WindowResult | null }>();
const emit = defineEmits<{ (e: "drill", node: TreeNode): void }>();

const hoveredId = ref<number | null>(null);

const WIDTH = 720;
const HEIGHT = 360;
const PADDING = 2; // daireler arası min mesafe
const ITER = 80;
const MIN_RADIUS = 4;

function formatBytes(b: number): string {
  if (b <= 0) return "—";
  const units = ["B", "KB", "MB", "GB", "TB"];
  let v = b;
  let i = 0;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i++;
  }
  return `${v.toFixed(i >= 3 ? 1 : 0)} ${units[i]}`;
}

function scoreTierColor(score: number | null): string {
  if (score === null) return "hsl(190 35% 38%)";
  if (score <= 30) return "#7f1d1d";
  if (score <= 60) return "#78350f";
  if (score <= 85) return "#14532d";
  return "#1e3a8a";
}

function scoreTierLabel(score: number | null): string {
  if (score === null) return "";
  if (score <= 30) return "DOKUNMA";
  if (score <= 60) return "İNCELE";
  if (score <= 85) return "BÜYÜK İHTİMAL";
  return "CACHE";
}

/**
 * Force relaxation pass: O(N²). N≤200 için ~40k op/iter × 80 = 3.2M, hızlı.
 * Çakışan çiftleri iter boyunca uzaklaştırır, merkeze hafif çeker.
 */
function relax(bubbles: Bubble[], cx: number, cy: number) {
  const n = bubbles.length;
  for (let pass = 0; pass < ITER; pass++) {
    // Çakışma itme
    for (let i = 0; i < n; i++) {
      for (let j = i + 1; j < n; j++) {
        const a = bubbles[i];
        const b = bubbles[j];
        const dx = b.x - a.x;
        const dy = b.y - a.y;
        const dist = Math.sqrt(dx * dx + dy * dy) || 0.0001;
        const minDist = a.r + b.r + PADDING;
        if (dist < minDist) {
          const overlap = (minDist - dist) / 2;
          const ux = dx / dist;
          const uy = dy / dist;
          a.x -= ux * overlap;
          a.y -= uy * overlap;
          b.x += ux * overlap;
          b.y += uy * overlap;
        }
      }
    }
    // Merkez çekimi (hafif)
    const pull = 0.04;
    for (const b of bubbles) {
      b.x += (cx - b.x) * pull;
      b.y += (cy - b.y) * pull;
    }
  }
}

const bubbles = computed<Bubble[]>(() => {
  if (!props.data || props.data.nodes.length === 0) return [];
  const total = props.data.parent_aggregate_size;
  if (total <= 0) return [];

  // Renk + boyut hazırla, en büyükten en küçüğe sırala.
  const canvasArea = WIDTH * HEIGHT * 0.55; // ~%55 doluluk hedefi
  const items = props.data.nodes
    .filter((n) => n.aggregate_size > 0)
    .map((n) => {
      const area = (n.aggregate_size / total) * canvasArea;
      const r = Math.max(MIN_RADIUS, Math.sqrt(area / Math.PI));
      return { node: n, r };
    })
    .filter((it) => it.r >= MIN_RADIUS)
    .sort((a, b) => b.r - a.r);

  if (items.length === 0) return [];

  const cx = WIDTH / 2;
  const cy = HEIGHT / 2;
  const goldenAngle = Math.PI * (3 - Math.sqrt(5)); // ≈ 137.5°

  const list: Bubble[] = items.map((it, i) => {
    // Spiral seed: en büyük merkez (i=0), diğerleri vogel spiral.
    let x = cx;
    let y = cy;
    if (i > 0) {
      const angle = i * goldenAngle;
      const radius = Math.sqrt(i) * 18;
      x = cx + Math.cos(angle) * radius;
      y = cy + Math.sin(angle) * radius;
    }
    return {
      id: it.node.id,
      name: it.node.name,
      size: it.node.aggregate_size,
      is_dir: it.node.is_dir,
      x,
      y,
      r: it.r,
      fill: scoreTierColor(it.node.score),
      scoreLabel: scoreTierLabel(it.node.score),
    };
  });

  relax(list, cx, cy);
  return list;
});

const hoveredBubble = computed<Bubble | null>(() => {
  if (hoveredId.value === null) return null;
  return bubbles.value.find((b) => b.id === hoveredId.value) ?? null;
});

function onClick(b: Bubble) {
  if (!b.is_dir) return;
  const node = props.data?.nodes.find((n) => n.id === b.id);
  if (node) emit("drill", node);
}

function percentOf(b: Bubble): string {
  if (!props.data || props.data.parent_aggregate_size <= 0) return "—";
  const pct = (b.size / props.data.parent_aggregate_size) * 100;
  if (pct < 0.1) return "<0.1%";
  return `${pct.toFixed(1)}%`;
}

function showLabel(b: Bubble): boolean {
  return b.r >= 22;
}

function labelTrunc(name: string, r: number): string {
  const max = Math.max(6, Math.floor(r / 4));
  return name.length > max ? name.slice(0, max - 1) + "…" : name;
}
</script>

<template>
  <div class="bubble-wrap">
    <svg
      v-if="data && bubbles.length"
      class="bubble-svg"
      :viewBox="`0 0 ${WIDTH} ${HEIGHT}`"
      preserveAspectRatio="xMidYMid meet"
    >
      <g>
        <g
          v-for="b in bubbles"
          :key="b.id"
          :class="{ bubble: true, 'bubble-dir': b.is_dir, 'bubble-active': hoveredId === b.id }"
          @mouseenter="hoveredId = b.id"
          @mouseleave="hoveredId = null"
          @click="onClick(b)"
        >
          <circle
            :cx="b.x"
            :cy="b.y"
            :r="b.r"
            :fill="b.fill"
            class="bubble-circle"
          />
          <text
            v-if="showLabel(b)"
            :x="b.x"
            :y="b.y - 2"
            text-anchor="middle"
            dominant-baseline="middle"
            :font-size="Math.min(12, Math.max(9, b.r / 4))"
            class="bubble-name"
          >
            {{ labelTrunc(b.name, b.r) }}
          </text>
          <text
            v-if="showLabel(b) && b.r >= 32"
            :x="b.x"
            :y="b.y + Math.min(12, b.r / 3)"
            text-anchor="middle"
            dominant-baseline="middle"
            :font-size="Math.max(8, b.r / 6)"
            class="bubble-size"
          >
            {{ formatBytes(b.size) }}
          </text>
          <title>
            {{ b.name }} · {{ formatBytes(b.size) }} · {{ percentOf(b) }}
          </title>
        </g>
      </g>
    </svg>
    <div v-else class="bubble-empty">Bubble için yeterli veri yok.</div>

    <div v-if="hoveredBubble" class="bubble-readout mono">
      <span class="readout-name">{{ hoveredBubble.name }}</span>
      <span class="readout-size">{{ formatBytes(hoveredBubble.size) }}</span>
      <span class="readout-pct">{{ percentOf(hoveredBubble) }}</span>
      <span v-if="hoveredBubble.scoreLabel" class="readout-score">
        {{ hoveredBubble.scoreLabel }}
      </span>
    </div>
  </div>
</template>

<style scoped>
.bubble-wrap {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 12px 0 4px;
}

.bubble-svg {
  width: 100%;
  height: auto;
  background: radial-gradient(circle at 50% 50%, #0f1318, #0b0d10);
  border-radius: 10px;
  user-select: none;
}

.bubble-circle {
  stroke: #0b0d10;
  stroke-width: 1.4;
  transition: opacity 0.15s, filter 0.15s;
  opacity: 0.85;
}

.bubble-dir .bubble-circle {
  cursor: pointer;
}

.bubble-dir:hover .bubble-circle,
.bubble-active .bubble-circle {
  opacity: 1;
  filter: brightness(1.3)
    drop-shadow(0 0 8px rgba(36, 200, 219, 0.5));
}

.bubble-name {
  fill: #f5f5f5;
  font-family: ui-monospace, monospace;
  pointer-events: none;
  text-shadow: 0 0 4px rgba(0, 0, 0, 0.8);
}

.bubble-size {
  fill: #d1d5db;
  font-family: ui-monospace, monospace;
  pointer-events: none;
  text-shadow: 0 0 4px rgba(0, 0, 0, 0.8);
}

.bubble-empty {
  padding: 60px 0;
  color: var(--muted);
  font-size: 13px;
  text-align: center;
}

.bubble-readout {
  display: flex;
  align-items: center;
  gap: 14px;
  padding: 6px 10px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--bg);
  font-size: 12px;
  flex-wrap: wrap;
}

.readout-name {
  color: var(--fg);
  font-weight: 500;
}

.readout-size {
  color: #6ee7b7;
}

.readout-pct {
  color: var(--muted);
}

.readout-score {
  color: #fcd34d;
  letter-spacing: 0.04em;
  font-weight: 600;
}
</style>
