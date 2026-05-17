<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<!--
  LiveSunburst — Sprint 3.7 (Bölüm 9.6.5 + 15.4) + 2026-05-17 polish.

  Tarama sırasında backend `scan-progress` event'leri her 10k entry'de
  `partial_tree: PartialNode[]` taşır. Bu component partial tree'yi alıp
  saf SVG arc render eder. d3 yok — el yazımı trig.

  Polish:
    * Küratörlü palet (Tableau 10) + depth tonlama (depth 2 daha koyu).
    * Hover'da wedge öne çıkar (scale 1.04 + parlaklık).
    * Tıklanınca wedge "seçili" olur (beyaz halka + öne) ve detay chip'i
      altta görünür. Aynı wedge'i tekrar tıklamak seçimi iptal eder.
    * `select` event'i parent'a yansır (ScanProgress.vue benimser).
-->
<script setup lang="ts">
import { computed, ref } from "vue";

interface PartialNode {
  id: number;
  parent: number | null;
  name: string;
  aggregate_size: number;
  depth: number;
  is_dir: boolean;
}

interface Wedge {
  id: number;
  name: string;
  size: number;
  depth: number;
  is_dir: boolean;
  path: string;
  color: string;
  edgeColor: string;
  labelX: number;
  labelY: number;
  showLabel: boolean;
  // Seçili wedge halo render'ı için aynı geometri başka renkle.
  midAngle: number;
  innerR: number;
  outerR: number;
}

export interface SunburstSelection {
  id: number;
  name: string;
  size: number;
  depth: number;
  is_dir: boolean;
}

const props = defineProps<{
  partialTree: PartialNode[];
  emptyMessage?: string;
}>();

const emit = defineEmits<{
  (e: "select", w: SunburstSelection | null): void;
}>();

// Görsel kanvas geometrisi (360×360 hedef).
const SIZE = 360;
const CENTER = SIZE / 2;
// Halka yarıçapları: depth 0 (root) merkez disk, depth 1 iç halka, depth 2 dış halka.
const RINGS: Array<{ inner: number; outer: number }> = [
  { inner: 0, outer: 40 },
  { inner: 48, outer: 110 },
  { inner: 118, outer: 170 },
];
const GAP_RAD = 0.004;

// Tableau 10 — perceptually balanced, dark/light tema'da okunabilir.
const PALETTE = [
  "#4e79a7",
  "#f28e2c",
  "#e15759",
  "#76b7b2",
  "#59a14f",
  "#edc949",
  "#af7aa1",
  "#ff9da7",
  "#9c755f",
  "#bab0ab",
  "#5b8ff9",
  "#5ad8a6",
];

const selectedId = ref<number | null>(null);

function polarToCart(angle: number, r: number): [number, number] {
  const a = angle - Math.PI / 2;
  return [Math.cos(a) * r, Math.sin(a) * r];
}

function arcPath(a1: number, a2: number, innerR: number, outerR: number): string {
  if (a2 <= a1) return "";
  const [ix1, iy1] = polarToCart(a1, innerR);
  const [ox1, oy1] = polarToCart(a1, outerR);
  const [ox2, oy2] = polarToCart(a2, outerR);
  const [ix2, iy2] = polarToCart(a2, innerR);
  const largeArc = a2 - a1 > Math.PI ? 1 : 0;
  if (innerR <= 0.001) {
    return [
      `M 0 0`,
      `L ${ox1.toFixed(2)} ${oy1.toFixed(2)}`,
      `A ${outerR} ${outerR} 0 ${largeArc} 1 ${ox2.toFixed(2)} ${oy2.toFixed(2)}`,
      "Z",
    ].join(" ");
  }
  return [
    `M ${ix1.toFixed(2)} ${iy1.toFixed(2)}`,
    `L ${ox1.toFixed(2)} ${oy1.toFixed(2)}`,
    `A ${outerR} ${outerR} 0 ${largeArc} 1 ${ox2.toFixed(2)} ${oy2.toFixed(2)}`,
    `L ${ix2.toFixed(2)} ${iy2.toFixed(2)}`,
    `A ${innerR} ${innerR} 0 ${largeArc} 0 ${ix1.toFixed(2)} ${iy1.toFixed(2)}`,
    "Z",
  ].join(" ");
}

function hexToRgb(hex: string): [number, number, number] {
  const v = parseInt(hex.slice(1), 16);
  return [(v >> 16) & 255, (v >> 8) & 255, v & 255];
}

function rgbToHex(r: number, g: number, b: number): string {
  const c = (r << 16) | (g << 8) | b;
  return `#${c.toString(16).padStart(6, "0")}`;
}

function mix(hex: string, target: string, factor: number): string {
  const [r1, g1, b1] = hexToRgb(hex);
  const [r2, g2, b2] = hexToRgb(target);
  return rgbToHex(
    Math.round(r1 + (r2 - r1) * factor),
    Math.round(g1 + (g2 - g1) * factor),
    Math.round(b1 + (b2 - b1) * factor),
  );
}

function colorFor(index: number, depth: number, isDir: boolean): string {
  const base = PALETTE[index % PALETTE.length];
  // depth 2 daha koyu (uzaktaymış gibi); dosya (non-dir) biraz daha desature.
  let color = base;
  if (depth === 2) color = mix(color, "#000000", 0.22);
  if (!isDir) color = mix(color, "#888888", 0.18);
  return color;
}

function edgeFor(fill: string): string {
  // Wedge kenarı için %35 koyu ton — 3D hissi.
  return mix(fill, "#000000", 0.35);
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
  return `${v.toFixed(i >= 3 ? 1 : 0)} ${units[i]}`;
}

const rootNode = computed<PartialNode | null>(() => {
  if (!props.partialTree || props.partialTree.length === 0) return null;
  return props.partialTree.find((n) => n.depth === 0) ?? null;
});

const isEmpty = computed(
  () =>
    !props.partialTree ||
    props.partialTree.length === 0 ||
    !rootNode.value ||
    rootNode.value.aggregate_size <= 0,
);

const wedges = computed<Wedge[]>(() => {
  if (isEmpty.value || !rootNode.value) return [];
  const total = rootNode.value.aggregate_size;
  if (total <= 0) return [];

  const root = rootNode.value;
  const depth1: PartialNode[] = props.partialTree
    .filter((n) => n.depth === 1 && n.parent === root.id && n.aggregate_size > 0)
    .sort((a, b) => b.aggregate_size - a.aggregate_size);

  const out: Wedge[] = [];

  let angle = 0;
  const parentSlots: Map<number, { a1: number; a2: number; index: number }> =
    new Map();

  for (let i = 0; i < depth1.length; i++) {
    const node = depth1[i];
    const ratio = node.aggregate_size / total;
    const a1 = angle;
    const a2 = angle + ratio * 2 * Math.PI;
    angle = a2;
    parentSlots.set(node.id, { a1, a2, index: i });

    const a1g = a1 + GAP_RAD / 2;
    const a2g = a2 - GAP_RAD / 2;
    if (a2g <= a1g) continue;

    const ring = RINGS[1];
    const labelAngle = (a1 + a2) / 2;
    const labelR = (ring.inner + ring.outer) / 2;
    const [lx, ly] = polarToCart(labelAngle, labelR);
    const fill = colorFor(i, 1, node.is_dir);
    out.push({
      id: node.id,
      name: node.name,
      size: node.aggregate_size,
      depth: 1,
      is_dir: node.is_dir,
      path: arcPath(a1g, a2g, ring.inner, ring.outer),
      color: fill,
      edgeColor: edgeFor(fill),
      labelX: lx,
      labelY: ly,
      showLabel: a2 - a1 > 0.22,
      midAngle: labelAngle,
      innerR: ring.inner,
      outerR: ring.outer,
    });
  }

  const depth2ByParent: Map<number, PartialNode[]> = new Map();
  for (const n of props.partialTree) {
    if (n.depth !== 2 || n.parent === null || n.aggregate_size <= 0) continue;
    if (!parentSlots.has(n.parent)) continue;
    const arr = depth2ByParent.get(n.parent) ?? [];
    arr.push(n);
    depth2ByParent.set(n.parent, arr);
  }

  for (const [parentId, kids] of depth2ByParent.entries()) {
    const slot = parentSlots.get(parentId);
    if (!slot) continue;
    const parentNode = depth1.find((p) => p.id === parentId);
    if (!parentNode || parentNode.aggregate_size <= 0) continue;
    const sortedKids = [...kids].sort((a, b) => b.aggregate_size - a.aggregate_size);
    let kidAngle = slot.a1;
    const slotSpan = slot.a2 - slot.a1;
    for (const k of sortedKids) {
      const ratio = k.aggregate_size / parentNode.aggregate_size;
      const span = ratio * slotSpan;
      const a1 = kidAngle;
      const a2 = kidAngle + span;
      kidAngle = a2;
      const a1g = a1 + GAP_RAD / 2;
      const a2g = a2 - GAP_RAD / 2;
      if (a2g <= a1g) continue;

      const ring = RINGS[2];
      const labelAngle = (a1 + a2) / 2;
      const labelR = (ring.inner + ring.outer) / 2;
      const [lx, ly] = polarToCart(labelAngle, labelR);
      // depth=2 wedge'in rengi parent'ın paletinden, koyulaştırılmış.
      const fill = colorFor(slot.index, 2, k.is_dir);
      out.push({
        id: k.id,
        name: k.name,
        size: k.aggregate_size,
        depth: 2,
        is_dir: k.is_dir,
        path: arcPath(a1g, a2g, ring.inner, ring.outer),
        color: fill,
        edgeColor: edgeFor(fill),
        labelX: lx,
        labelY: ly,
        showLabel: a2 - a1 > 0.18,
        midAngle: labelAngle,
        innerR: ring.inner,
        outerR: ring.outer,
      });
    }
  }

  return out;
});

const totalLabel = computed(() => {
  if (!rootNode.value) return "—";
  return formatBytes(rootNode.value.aggregate_size);
});

const selectedWedge = computed<Wedge | null>(() => {
  if (selectedId.value === null) return null;
  return wedges.value.find((w) => w.id === selectedId.value) ?? null;
});

function onWedgeClick(w: Wedge): void {
  if (selectedId.value === w.id) {
    selectedId.value = null;
    emit("select", null);
    return;
  }
  selectedId.value = w.id;
  emit("select", {
    id: w.id,
    name: w.name,
    size: w.size,
    depth: w.depth,
    is_dir: w.is_dir,
  });
}
</script>

<template>
  <div class="live-sunburst-wrap">
    <svg
      v-if="!isEmpty"
      class="live-sunburst-svg"
      :viewBox="`${-CENTER} ${-CENTER} ${SIZE} ${SIZE}`"
      preserveAspectRatio="xMidYMid meet"
      role="img"
      aria-label="Canlı tarama haritası"
    >
      <defs>
        <!-- 3D hissi için tek bir radial overlay: orta dışa açılan beyaz
             cila tüm wedge'lere uygulanır (multiply olmadan blend) -->
        <radialGradient id="sun-gloss" cx="50%" cy="50%" r="55%">
          <stop offset="0%" stop-color="rgba(255,255,255,0.18)" />
          <stop offset="60%" stop-color="rgba(255,255,255,0.05)" />
          <stop offset="100%" stop-color="rgba(0,0,0,0.25)" />
        </radialGradient>
        <filter id="sun-shadow" x="-20%" y="-20%" width="140%" height="140%">
          <feDropShadow
            dx="0"
            dy="1.5"
            stdDeviation="1.2"
            flood-color="rgba(0,0,0,0.4)"
          />
        </filter>
      </defs>

      <circle :r="RINGS[0].outer" class="root-disk" />

      <g class="wedges" filter="url(#sun-shadow)">
        <path
          v-for="seg in wedges"
          :key="`w-${seg.id}`"
          :d="seg.path"
          :fill="seg.color"
          :stroke="seg.edgeColor"
          stroke-width="0.6"
          :class="[
            'wedge',
            `wedge-d${seg.depth}`,
            seg.is_dir ? 'wedge-dir' : 'wedge-file',
            selectedId === seg.id ? 'wedge-selected' : '',
          ]"
          @click.stop="onWedgeClick(seg)"
        >
          <title>{{ seg.name }} · {{ formatBytes(seg.size) }}</title>
        </path>
      </g>

      <!-- 3D parlaklık katmanı tüm wedge'lerin üstünde -->
      <circle
        :r="RINGS[2].outer"
        fill="url(#sun-gloss)"
        class="gloss-overlay"
      />

      <g class="labels" pointer-events="none">
        <text
          v-for="seg in wedges"
          :key="`l-${seg.id}`"
          v-show="seg.showLabel"
          :x="seg.labelX"
          :y="seg.labelY"
          text-anchor="middle"
          dominant-baseline="middle"
          class="wedge-label"
        >
          {{ seg.name.length > 12 ? seg.name.slice(0, 11) + "…" : seg.name }}
        </text>
      </g>

      <!-- Center total -->
      <text
        x="0"
        y="-4"
        text-anchor="middle"
        dominant-baseline="middle"
        class="center-total"
        pointer-events="none"
      >
        {{ totalLabel }}
      </text>
      <text
        x="0"
        y="14"
        text-anchor="middle"
        dominant-baseline="middle"
        class="center-count"
        pointer-events="none"
      >
        {{ wedges.length }}
      </text>
    </svg>
    <div v-else class="live-sunburst-empty">
      <div class="empty-disk"></div>
      <p class="empty-text">{{ emptyMessage ?? "İlk veriler bekleniyor…" }}</p>
    </div>

    <!-- Tıklanan wedge için detay chip'i. Sunburst'un altında, panel
         içinde kalır. Tekrar tıklama veya başka wedge ile kapanır. -->
    <transition name="info-pop">
      <div
        v-if="selectedWedge"
        class="wedge-info"
        role="dialog"
        aria-live="polite"
      >
        <div class="wedge-info-row">
          <span
            class="wedge-info-swatch"
            :style="{ background: selectedWedge.color }"
          ></span>
          <span class="wedge-info-name mono">
            {{ selectedWedge.is_dir ? "📁" : "📄" }} {{ selectedWedge.name }}
          </span>
        </div>
        <div class="wedge-info-meta">
          <span class="wedge-info-size mono">{{ formatBytes(selectedWedge.size) }}</span>
          <span class="wedge-info-depth">
            {{ selectedWedge.depth === 1 ? "üst klasör" : "alt klasör" }}
          </span>
        </div>
      </div>
    </transition>
  </div>
</template>

<style scoped>
.live-sunburst-wrap {
  width: 360px;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  position: relative;
  flex-shrink: 0;
  gap: 10px;
}

.live-sunburst-svg {
  width: 360px;
  height: 360px;
  user-select: none;
  overflow: visible;
}

.root-disk {
  fill: var(--bg);
  stroke: var(--border);
  stroke-width: 1.2;
}

.wedges {
  cursor: pointer;
}

.wedge {
  opacity: 0.95;
  transition:
    opacity 0.4s ease-out,
    transform 0.18s cubic-bezier(0.34, 1.56, 0.64, 1),
    filter 0.18s ease-out,
    stroke 0.18s ease-out,
    stroke-width 0.18s ease-out;
  transform-origin: 0 0;
  transform-box: fill-box;
  animation: wedge-fade-in 0.32s ease-out;
}

@keyframes wedge-fade-in {
  from {
    opacity: 0;
    transform: scale(0.92);
  }
  to {
    opacity: 0.95;
    transform: scale(1);
  }
}

/* Hover: hafif öne çık + parla. */
.wedge:hover {
  opacity: 1;
  filter: brightness(1.12) drop-shadow(0 3px 6px rgba(0, 0, 0, 0.45));
  transform: scale(1.025);
}

/* Seçili wedge: beyaz halka + öne. */
.wedge-selected {
  opacity: 1;
  stroke: #ffffff;
  stroke-width: 2;
  filter: brightness(1.18) drop-shadow(0 4px 10px rgba(0, 0, 0, 0.55));
  transform: scale(1.045);
}

.gloss-overlay {
  pointer-events: none;
  mix-blend-mode: soft-light;
}

.wedge-label {
  font-size: 10px;
  fill: #f5f5f5;
  font-family: ui-monospace, "Cascadia Code", "Consolas", monospace;
  paint-order: stroke fill;
  stroke: rgba(0, 0, 0, 0.6);
  stroke-width: 2.2px;
  stroke-linejoin: round;
  pointer-events: none;
}

.center-total {
  font-size: 14px;
  fill: var(--fg);
  font-family: ui-monospace, monospace;
  font-weight: 600;
  paint-order: stroke fill;
  stroke: var(--surface);
  stroke-width: 3px;
  stroke-linejoin: round;
}

.center-count {
  font-size: 10px;
  fill: var(--muted);
  font-family: ui-monospace, monospace;
}

.live-sunburst-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 16px;
  color: var(--muted);
  min-height: 320px;
}

.empty-disk {
  width: 120px;
  height: 120px;
  border: 2px dashed var(--border);
  border-radius: 50%;
  animation: empty-pulse 1.6s ease-in-out infinite;
}

@keyframes empty-pulse {
  0%,
  100% {
    opacity: 0.4;
    transform: scale(0.95);
  }
  50% {
    opacity: 0.8;
    transform: scale(1);
  }
}

.empty-text {
  margin: 0;
  font-size: 12px;
  font-style: italic;
}

/* Tıklanan wedge detay chip'i (sunburst'un altında). */
.wedge-info {
  width: 100%;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 8px 12px;
  display: flex;
  flex-direction: column;
  gap: 6px;
  box-shadow: 0 6px 14px var(--shadow);
}

.wedge-info-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

.wedge-info-swatch {
  width: 12px;
  height: 12px;
  border-radius: 3px;
  border: 1px solid rgba(0, 0, 0, 0.25);
  flex-shrink: 0;
}

.wedge-info-name {
  font-size: 13px;
  color: var(--fg);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  min-width: 0;
  flex: 1;
}

.wedge-info-meta {
  display: flex;
  align-items: center;
  gap: 10px;
  font-size: 11px;
}

.wedge-info-size {
  color: var(--fg);
  font-weight: 600;
}

.wedge-info-depth {
  color: var(--muted);
  text-transform: uppercase;
  letter-spacing: 0.06em;
  font-size: 10px;
}

.info-pop-enter-from,
.info-pop-leave-to {
  opacity: 0;
  transform: translateY(-4px);
}
.info-pop-enter-active,
.info-pop-leave-active {
  transition: opacity 0.18s ease, transform 0.18s ease;
}

@media (prefers-reduced-motion: reduce) {
  .wedge,
  .info-pop-enter-active,
  .info-pop-leave-active {
    animation: none;
    transition: none;
  }
}
</style>
