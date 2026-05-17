<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<!--
  Sunburst donut — Bölüm 9.1 (dört görüntü modu) Pillar 2.
  Tek halka, mevcut WindowResult'tan render. Hover highlight + click drill.
  SVG-only, hand-rolled trig — bundle'a d3 eklenmedi.
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

interface Segment {
  id: number;
  name: string;
  is_dir: boolean;
  size: number;
  path: string;
  color: string;
  edgeColor: string;
  labelX: number;
  labelY: number;
  showLabel: boolean;
  angleSpan: number;
}

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

function hexToRgb(hex: string): [number, number, number] {
  const v = parseInt(hex.slice(1), 16);
  return [(v >> 16) & 255, (v >> 8) & 255, v & 255];
}

function mix(hex: string, target: string, factor: number): string {
  const [r1, g1, b1] = hexToRgb(hex);
  const [r2, g2, b2] = hexToRgb(target);
  const r = Math.round(r1 + (r2 - r1) * factor);
  const g = Math.round(g1 + (g2 - g1) * factor);
  const b = Math.round(b1 + (b2 - b1) * factor);
  return `#${((r << 16) | (g << 8) | b).toString(16).padStart(6, "0")}`;
}

const props = defineProps<{
  data: WindowResult | null;
}>();

const emit = defineEmits<{
  (e: "drill", node: TreeNode): void;
}>();

const hoveredId = ref<number | null>(null);

const INNER_R = 80;
const OUTER_R = 240;
const GAP_RAD = 0.002; // ~0.1° between arcs

function polarToCart(angle: number, r: number): [number, number] {
  // SVG y down; 0 at top → -π/2 offset
  const a = angle - Math.PI / 2;
  return [Math.cos(a) * r, Math.sin(a) * r];
}

function arcPath(
  a1: number,
  a2: number,
  innerR: number,
  outerR: number,
): string {
  if (a2 <= a1) return "";
  const [ix1, iy1] = polarToCart(a1, innerR);
  const [ox1, oy1] = polarToCart(a1, outerR);
  const [ox2, oy2] = polarToCart(a2, outerR);
  const [ix2, iy2] = polarToCart(a2, innerR);
  const largeArc = a2 - a1 > Math.PI ? 1 : 0;
  return [
    `M ${ix1.toFixed(2)} ${iy1.toFixed(2)}`,
    `L ${ox1.toFixed(2)} ${oy1.toFixed(2)}`,
    `A ${outerR} ${outerR} 0 ${largeArc} 1 ${ox2.toFixed(2)} ${oy2.toFixed(2)}`,
    `L ${ix2.toFixed(2)} ${iy2.toFixed(2)}`,
    `A ${innerR} ${innerR} 0 ${largeArc} 0 ${ix1.toFixed(2)} ${iy1.toFixed(2)}`,
    "Z",
  ].join(" ");
}

function colorFor(index: number, isDir: boolean): string {
  let color = PALETTE[index % PALETTE.length];
  if (!isDir) color = mix(color, "#888888", 0.18);
  return color;
}

function edgeFor(fill: string): string {
  return mix(fill, "#000000", 0.32);
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

const segments = computed<Segment[]>(() => {
  if (!props.data) return [];
  const total = props.data.parent_aggregate_size;
  if (total <= 0) return [];

  const segs: Segment[] = [];
  let angle = 0;
  const items = props.data.nodes;
  const n = items.length;

  for (let i = 0; i < n; i++) {
    const node = items[i];
    const ratio = node.aggregate_size / total;
    if (ratio <= 0) continue;
    const a1 = angle;
    const a2 = angle + ratio * 2 * Math.PI;
    angle = a2;
    const angleSpan = a2 - a1;
    const a1g = a1 + GAP_RAD / 2;
    const a2g = a2 - GAP_RAD / 2;
    if (a2g <= a1g) continue;

    const labelAngle = (a1 + a2) / 2;
    const labelR = (INNER_R + OUTER_R) / 2;
    const [lx, ly] = polarToCart(labelAngle, labelR);

    const fill = colorFor(i, node.is_dir);
    segs.push({
      id: node.id,
      name: node.name,
      is_dir: node.is_dir,
      size: node.aggregate_size,
      path: arcPath(a1g, a2g, INNER_R, OUTER_R),
      color: fill,
      edgeColor: edgeFor(fill),
      labelX: lx,
      labelY: ly,
      showLabel: angleSpan > 0.18, // ≥ ~10°
      angleSpan,
    });
  }
  return segs;
});

const hoveredSegment = computed<Segment | null>(() => {
  if (hoveredId.value === null) return null;
  return segments.value.find((s) => s.id === hoveredId.value) ?? null;
});

const centerLine1 = computed(() =>
  hoveredSegment.value
    ? hoveredSegment.value.name
    : (props.data?.parent_name ?? "—"),
);

const centerLine2 = computed(() =>
  hoveredSegment.value
    ? formatBytes(hoveredSegment.value.size)
    : formatBytes(props.data?.parent_aggregate_size ?? 0),
);

const centerLine3 = computed(() => {
  if (hoveredSegment.value && props.data) {
    const pct =
      (hoveredSegment.value.size / props.data.parent_aggregate_size) * 100;
    return `${pct.toFixed(1)}%`;
  }
  return props.data
    ? `${props.data.returned} / ${props.data.total_children}`
    : "";
});

function onClick(seg: Segment) {
  const node = props.data?.nodes.find((n) => n.id === seg.id);
  if (node) emit("drill", node);
}
</script>

<template>
  <div class="sunburst-wrap">
    <svg
      v-if="data && segments.length"
      class="sunburst-svg"
      :viewBox="`${-OUTER_R - 10} ${-OUTER_R - 10} ${OUTER_R * 2 + 20} ${OUTER_R * 2 + 20}`"
      preserveAspectRatio="xMidYMid meet"
    >
      <defs>
        <radialGradient id="db-sun-gloss" cx="50%" cy="50%" r="55%">
          <stop offset="0%" stop-color="rgba(255,255,255,0.18)" />
          <stop offset="60%" stop-color="rgba(255,255,255,0.05)" />
          <stop offset="100%" stop-color="rgba(0,0,0,0.28)" />
        </radialGradient>
        <filter
          id="db-sun-shadow"
          x="-20%"
          y="-20%"
          width="140%"
          height="140%"
        >
          <feDropShadow
            dx="0"
            dy="2"
            stdDeviation="1.6"
            flood-color="rgba(0,0,0,0.45)"
          />
        </filter>
      </defs>

      <g filter="url(#db-sun-shadow)">
        <path
          v-for="seg in segments"
          :key="seg.id"
          :d="seg.path"
          :fill="seg.color"
          :stroke="seg.edgeColor"
          stroke-width="0.6"
          :class="{
            arc: true,
            'arc-dir': seg.is_dir,
            'arc-active': hoveredId === seg.id,
          }"
          @mouseenter="hoveredId = seg.id"
          @mouseleave="hoveredId = null"
          @click="onClick(seg)"
        >
          <title>
            {{ seg.name }} · {{ formatBytes(seg.size) }}
          </title>
        </path>
      </g>

      <!-- 3D parlaklık katmanı tüm wedge'lerin üstünde -->
      <circle
        :r="OUTER_R"
        fill="url(#db-sun-gloss)"
        class="gloss-overlay"
      />

      <g pointer-events="none">
        <template v-for="seg in segments" :key="`l-${seg.id}`">
          <text
            v-if="seg.showLabel"
            :x="seg.labelX"
            :y="seg.labelY"
            text-anchor="middle"
            dominant-baseline="middle"
            class="arc-label"
          >
            {{ seg.name.length > 14 ? seg.name.slice(0, 13) + "…" : seg.name }}
          </text>
        </template>

        <circle :r="INNER_R - 4" class="center-disk" />
        <text
          x="0"
          y="-12"
          text-anchor="middle"
          dominant-baseline="middle"
          class="center-name"
        >
          {{
            centerLine1.length > 18
              ? centerLine1.slice(0, 17) + "…"
              : centerLine1
          }}
        </text>
        <text
          x="0"
          y="10"
          text-anchor="middle"
          dominant-baseline="middle"
          class="center-size"
        >
          {{ centerLine2 }}
        </text>
        <text
          x="0"
          y="30"
          text-anchor="middle"
          dominant-baseline="middle"
          class="center-pct"
        >
          {{ centerLine3 }}
        </text>
      </g>
    </svg>
    <div v-else class="sunburst-empty">Bu klasör altında veri yok.</div>
  </div>
</template>

<style scoped>
.sunburst-wrap {
  display: flex;
  justify-content: center;
  align-items: center;
  padding: 12px 0 20px;
}

.sunburst-svg {
  width: 100%;
  max-width: 480px;
  height: auto;
  user-select: none;
}

.arc {
  opacity: 0.95;
  transition:
    opacity 0.15s ease,
    filter 0.18s ease,
    transform 0.18s cubic-bezier(0.34, 1.56, 0.64, 1),
    stroke-width 0.18s ease;
  transform-origin: 0 0;
  transform-box: fill-box;
}

.arc-dir {
  cursor: pointer;
}

.arc-dir:hover,
.arc-active {
  opacity: 1;
  filter: brightness(1.14) drop-shadow(0 3px 7px rgba(0, 0, 0, 0.45));
  transform: scale(1.025);
  stroke-width: 1.4;
}

.gloss-overlay {
  pointer-events: none;
  mix-blend-mode: soft-light;
}

.arc-label {
  font-size: 11px;
  fill: #ffffff;
  font-family: ui-monospace, "Cascadia Code", "Consolas", monospace;
  paint-order: stroke fill;
  stroke: rgba(0, 0, 0, 0.65);
  stroke-width: 2.2px;
  stroke-linejoin: round;
  pointer-events: none;
}

.center-disk {
  fill: var(--surface);
  stroke: var(--border);
  stroke-width: 1.2;
}

.center-name {
  font-size: 13px;
  fill: var(--fg);
  font-weight: 600;
  paint-order: stroke fill;
  stroke: var(--surface);
  stroke-width: 3px;
  stroke-linejoin: round;
}

.center-size {
  font-size: 14px;
  fill: var(--fg);
  font-family: ui-monospace, monospace;
  font-weight: 600;
}

.center-pct {
  font-size: 11px;
  fill: var(--muted);
  font-family: ui-monospace, monospace;
}

@media (prefers-reduced-motion: reduce) {
  .arc {
    transition: none;
  }
}

.sunburst-empty {
  padding: 60px 0;
  color: var(--muted);
  font-size: 13px;
}
</style>
