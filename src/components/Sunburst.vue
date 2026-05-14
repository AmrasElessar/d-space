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
  labelX: number;
  labelY: number;
  showLabel: boolean;
  angleSpan: number;
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

function colorFor(index: number, total: number, isDir: boolean): string {
  const baseHue = 175;
  const span = 80;
  const hue = (baseHue + (index * span) / Math.max(total, 6)) % 360;
  const sat = isDir ? 50 : 35;
  const light = isDir ? 48 : 38;
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

    segs.push({
      id: node.id,
      name: node.name,
      is_dir: node.is_dir,
      size: node.aggregate_size,
      path: arcPath(a1g, a2g, INNER_R, OUTER_R),
      color: colorFor(i, n, node.is_dir),
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
      <g>
        <path
          v-for="seg in segments"
          :key="seg.id"
          :d="seg.path"
          :fill="seg.color"
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

        <circle :r="INNER_R - 4" fill="#0b0d10" />
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
  stroke: #0b0d10;
  stroke-width: 1.2;
  transition: opacity 0.15s, filter 0.15s;
  opacity: 0.85;
}

.arc-dir {
  cursor: pointer;
}

.arc-dir:hover,
.arc-active {
  opacity: 1;
  filter: drop-shadow(0 0 6px rgba(36, 200, 219, 0.4));
}

.arc-label {
  font-size: 10px;
  fill: #f5f5f5;
  font-family: ui-monospace, monospace;
  pointer-events: none;
  text-shadow: 0 0 4px rgba(0, 0, 0, 0.6);
}

.center-name {
  font-size: 13px;
  fill: var(--fg);
  font-weight: 600;
}

.center-size {
  font-size: 14px;
  fill: #6ee7b7;
  font-family: ui-monospace, monospace;
}

.center-pct {
  font-size: 11px;
  fill: var(--muted);
  font-family: ui-monospace, monospace;
}

.sunburst-empty {
  padding: 60px 0;
  color: var(--muted);
  font-size: 13px;
}
</style>
