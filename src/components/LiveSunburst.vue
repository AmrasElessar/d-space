<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<!--
  LiveSunburst — Sprint 3.7 (Bölüm 9.6.5 + 15.4).

  Tarama sırasında backend `scan-progress` event'leri her 10k entry'de
  `partial_tree: PartialNode[]` taşır. Bu component partial tree'yi alıp
  saf SVG arc render eder. d3 yok — el yazımı trig (Sunburst.vue ile
  aynı yaklaşım, daha sade tek-halka düzen).

  Akıcılık: her arc CSS `transition: d 0.4s` ile değil (SVG `d`
  doğrudan tween edilmez), bunun yerine yeni gelen wedge'ler opacity
  0 → 1 fade'in. Mevcut wedge'lerin açıları her event'te yeniden
  hesaplanır, browser path repaint eder.
-->
<script setup lang="ts">
import { computed } from "vue";

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
  labelX: number;
  labelY: number;
  showLabel: boolean;
}

const props = defineProps<{
  partialTree: PartialNode[];
  emptyMessage?: string;
}>();

// Görsel kanvas geometrisi (360×360 hedef).
const SIZE = 360;
const CENTER = SIZE / 2;
// Halka yarıçapları: depth 0 (root) merkez disk, depth 1 iç halka, depth 2 dış halka.
const RINGS: Array<{ inner: number; outer: number }> = [
  { inner: 0, outer: 40 }, // root center disk
  { inner: 48, outer: 110 }, // depth 1
  { inner: 118, outer: 170 }, // depth 2
];
const GAP_RAD = 0.004;

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
    // Pie slice (root disk yok zaten ama defensive).
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

function colorFor(index: number, total: number, depth: number, isDir: boolean): string {
  const baseHue = 175;
  const span = 120;
  const hue = (baseHue + (index * span) / Math.max(total, 6)) % 360;
  const sat = isDir ? 55 : 38;
  const light = depth === 1 ? 48 : 38;
  return `hsl(${hue.toFixed(0)} ${sat}% ${light}%)`;
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

// Root düğümünü ayıklar (depth=0). Yoksa null.
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

// Wedge inşası — depth 1 ve depth 2.
// Her depth seviyesinde parent'ın aggregate'i 360° olarak normalize edilir.
// Depth 1: root'un aggregate'i baz, parent=root tüm wedge'leri tam çember.
// Depth 2: parent (depth=1) düğümünün aggregate'i baz, kendi yay aralığında.
const wedges = computed<Wedge[]>(() => {
  if (isEmpty.value || !rootNode.value) return [];
  const total = rootNode.value.aggregate_size;
  if (total <= 0) return [];

  const root = rootNode.value;
  const depth1: PartialNode[] = props.partialTree
    .filter((n) => n.depth === 1 && n.parent === root.id && n.aggregate_size > 0)
    .sort((a, b) => b.aggregate_size - a.aggregate_size);

  const wedges: Wedge[] = [];

  // Depth 1 wedge'leri root'un aggregate'i üzerinden 360°'ye yayılır.
  // Eğer depth1'in toplamı root'tan küçükse, kalan boşluk olarak görünür
  // (ham agrega tutarlılığı).
  let angle = 0;
  const n1 = depth1.length;
  // depth=1 düğümler için (id → [a1, a2]) sözlüğü; depth=2 wedge'leri
  // parent'ın yay aralığını kullanır.
  const parentSlots: Map<number, { a1: number; a2: number }> = new Map();

  for (let i = 0; i < n1; i++) {
    const node = depth1[i];
    const ratio = node.aggregate_size / total;
    const a1 = angle;
    const a2 = angle + ratio * 2 * Math.PI;
    angle = a2;
    parentSlots.set(node.id, { a1, a2 });

    const a1g = a1 + GAP_RAD / 2;
    const a2g = a2 - GAP_RAD / 2;
    if (a2g <= a1g) continue;

    const ring = RINGS[1];
    const labelAngle = (a1 + a2) / 2;
    const labelR = (ring.inner + ring.outer) / 2;
    const [lx, ly] = polarToCart(labelAngle, labelR);
    wedges.push({
      id: node.id,
      name: node.name,
      size: node.aggregate_size,
      depth: 1,
      is_dir: node.is_dir,
      path: arcPath(a1g, a2g, ring.inner, ring.outer),
      color: colorFor(i, n1, 1, node.is_dir),
      labelX: lx,
      labelY: ly,
      showLabel: a2 - a1 > 0.22,
    });
  }

  // Depth 2 wedge'leri — parent'ın yay aralığı içinde parent.aggregate'e oranlanır.
  // partialTree depth=2 düğümleri parent.id'lere göre gruplandırılır.
  const depth2ByParent: Map<number, PartialNode[]> = new Map();
  for (const n of props.partialTree) {
    if (n.depth !== 2 || n.parent === null || n.aggregate_size <= 0) continue;
    if (!parentSlots.has(n.parent)) continue;
    const arr = depth2ByParent.get(n.parent) ?? [];
    arr.push(n);
    depth2ByParent.set(n.parent, arr);
  }

  let d2Index = 0;
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
      wedges.push({
        id: k.id,
        name: k.name,
        size: k.aggregate_size,
        depth: 2,
        is_dir: k.is_dir,
        path: arcPath(a1g, a2g, ring.inner, ring.outer),
        color: colorFor(d2Index, 24, 2, k.is_dir),
        labelX: lx,
        labelY: ly,
        showLabel: a2 - a1 > 0.18,
      });
      d2Index++;
    }
  }

  return wedges;
});

const totalLabel = computed(() => {
  if (!rootNode.value) return "—";
  return formatBytes(rootNode.value.aggregate_size);
});
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
      <!-- Root center disk -->
      <circle :r="RINGS[0].outer" class="root-disk" />

      <g class="wedges">
        <path
          v-for="seg in wedges"
          :key="`w-${seg.id}`"
          :d="seg.path"
          :fill="seg.color"
          :class="['wedge', `wedge-d${seg.depth}`, seg.is_dir ? 'wedge-dir' : 'wedge-file']"
        >
          <title>{{ seg.name }} · {{ formatBytes(seg.size) }}</title>
        </path>
      </g>

      <g class="labels">
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
      <text x="0" y="-4" text-anchor="middle" dominant-baseline="middle" class="center-total">
        {{ totalLabel }}
      </text>
      <text x="0" y="14" text-anchor="middle" dominant-baseline="middle" class="center-count">
        {{ wedges.length }}
      </text>
    </svg>
    <div v-else class="live-sunburst-empty">
      <div class="empty-disk"></div>
      <p class="empty-text">{{ emptyMessage ?? "İlk veriler bekleniyor…" }}</p>
    </div>
  </div>
</template>

<style scoped>
.live-sunburst-wrap {
  width: 360px;
  height: 360px;
  display: flex;
  align-items: center;
  justify-content: center;
  position: relative;
  flex-shrink: 0;
}

.live-sunburst-svg {
  width: 100%;
  height: 100%;
  user-select: none;
}

.root-disk {
  fill: #0b0d10;
  stroke: var(--border);
  stroke-width: 1.2;
}

.wedge {
  stroke: #0b0d10;
  stroke-width: 1;
  opacity: 0.92;
  transition:
    opacity 0.4s ease-out,
    fill 0.4s ease-out;
  animation: wedge-fade-in 0.4s ease-out;
}

@keyframes wedge-fade-in {
  from {
    opacity: 0;
  }
  to {
    opacity: 0.92;
  }
}

.wedge-d1 {
  filter: drop-shadow(0 0 2px rgba(36, 200, 219, 0.15));
}

.wedge-label {
  font-size: 9px;
  fill: #f5f5f5;
  font-family: ui-monospace, monospace;
  pointer-events: none;
  text-shadow: 0 0 4px rgba(0, 0, 0, 0.6);
}

.center-total {
  font-size: 13px;
  fill: #6ee7b7;
  font-family: ui-monospace, monospace;
  font-weight: 600;
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
</style>
