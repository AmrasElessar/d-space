<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<!--
  Treemap — Bölüm 9.1 mod 2/4 (dört görüntü modu), Pillar 2 Görsel Zarafet.

  Squarified layout (Bruls, Huijsen, van Wijk 2000) — alan ile orantılı
  dikdörtgenler, aspect ratio en kareye yakın olacak şekilde gruplanır.
  Hand-rolled, D3 yok (bundle bütçesi aynı).

  Renk: Bölüm 6.3 skor tier renk kodlaması (danger/caution/likely/cache);
  skor yoksa nötr teal gradyan. Click → drill emit (sadece klasörler).
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

interface Cell {
  id: number;
  name: string;
  size: number;
  is_dir: boolean;
  x: number;
  y: number;
  w: number;
  h: number;
  fill: string;
  showLabel: boolean;
  labelFontSize: number;
  scoreLabel: string;
}

const props = defineProps<{ data: WindowResult | null }>();
const emit = defineEmits<{ (e: "drill", node: TreeNode): void }>();

const hoveredId = ref<number | null>(null);

const WIDTH = 720;
const HEIGHT = 360;
const PAD = 2; // hücreler arası boşluk
const MIN_AREA = 16; // 16 px² altı render edilmez

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
  if (score === null) return "hsl(190 35% 38%)"; // nötr teal
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

interface SizedNode {
  node: TreeNode;
  area: number; // pixel cinsinden hedef alan
}

interface Rect {
  x: number;
  y: number;
  w: number;
  h: number;
}

/**
 * Squarified algoritma — Bruls/Huijsen/van Wijk 2000.
 * Items boyuta göre azalan sırada gelmeli.
 */
function squarify(items: SizedNode[], rect: Rect): Cell[] {
  const out: Cell[] = [];
  const queue = items.slice();
  let current: Rect = { ...rect };

  while (queue.length > 0) {
    const row: SizedNode[] = [];
    let bestWorst = Infinity;
    const shortSide = Math.min(current.w, current.h);
    if (shortSide <= 0) break;

    while (queue.length > 0) {
      const candidate = [...row, queue[0]];
      const w = worst(candidate, shortSide);
      if (row.length === 0 || w <= bestWorst) {
        bestWorst = w;
        row.push(queue.shift()!);
      } else {
        break;
      }
    }

    const placed = layoutRow(row, current);
    out.push(...placed.cells);
    current = placed.remaining;
    if (current.w <= 0 || current.h <= 0) break;
  }

  return out;
}

function worst(row: SizedNode[], w: number): number {
  if (row.length === 0) return Infinity;
  let s = 0;
  let rMax = 0;
  let rMin = Infinity;
  for (const it of row) {
    s += it.area;
    if (it.area > rMax) rMax = it.area;
    if (it.area < rMin) rMin = it.area;
  }
  if (s <= 0 || rMin <= 0) return Infinity;
  const w2 = w * w;
  const s2 = s * s;
  return Math.max((w2 * rMax) / s2, s2 / (w2 * rMin));
}

function layoutRow(
  row: SizedNode[],
  rect: Rect,
): { cells: Cell[]; remaining: Rect } {
  const cells: Cell[] = [];
  const sum = row.reduce((acc, r) => acc + r.area, 0);
  if (sum <= 0 || row.length === 0) {
    return { cells, remaining: rect };
  }

  const horizontal = rect.w <= rect.h;
  const rowThickness = sum / (horizontal ? rect.w : rect.h);
  let cursor = horizontal ? rect.x : rect.y;

  for (const item of row) {
    const length = item.area / rowThickness;
    if (horizontal) {
      cells.push(
        makeCell(item.node, cursor, rect.y, length, rowThickness, item.area),
      );
    } else {
      cells.push(
        makeCell(item.node, rect.x, cursor, rowThickness, length, item.area),
      );
    }
    cursor += length;
  }

  const remaining: Rect = horizontal
    ? {
        x: rect.x,
        y: rect.y + rowThickness,
        w: rect.w,
        h: rect.h - rowThickness,
      }
    : {
        x: rect.x + rowThickness,
        y: rect.y,
        w: rect.w - rowThickness,
        h: rect.h,
      };

  return { cells, remaining };
}

function makeCell(
  node: TreeNode,
  rawX: number,
  rawY: number,
  rawW: number,
  rawH: number,
  _area: number,
): Cell {
  const innerW = Math.max(rawW - PAD, 0);
  const innerH = Math.max(rawH - PAD, 0);
  const x = rawX + PAD / 2;
  const y = rawY + PAD / 2;
  const showLabel = innerW >= 60 && innerH >= 22;
  const labelFontSize = Math.min(13, Math.max(9, Math.floor(innerH / 5)));
  return {
    id: node.id,
    name: node.name,
    size: node.aggregate_size,
    is_dir: node.is_dir,
    x,
    y,
    w: innerW,
    h: innerH,
    fill: scoreTierColor(node.score),
    showLabel,
    labelFontSize,
    scoreLabel: scoreTierLabel(node.score),
  };
}

const cells = computed<Cell[]>(() => {
  if (!props.data || props.data.nodes.length === 0) return [];
  const total = props.data.parent_aggregate_size;
  if (total <= 0) return [];

  const canvasArea = WIDTH * HEIGHT;
  const items: SizedNode[] = props.data.nodes
    .filter((n) => n.aggregate_size > 0)
    .map((n) => ({ node: n, area: (n.aggregate_size / total) * canvasArea }))
    .filter((it) => it.area >= MIN_AREA)
    .sort((a, b) => b.area - a.area);

  if (items.length === 0) return [];
  return squarify(items, { x: 0, y: 0, w: WIDTH, h: HEIGHT });
});

const hoveredCell = computed<Cell | null>(() => {
  if (hoveredId.value === null) return null;
  return cells.value.find((c) => c.id === hoveredId.value) ?? null;
});

function onClick(cell: Cell) {
  if (!cell.is_dir) return;
  const node = props.data?.nodes.find((n) => n.id === cell.id);
  if (node) emit("drill", node);
}

function percentOf(cell: Cell): string {
  if (!props.data || props.data.parent_aggregate_size <= 0) return "—";
  const pct = (cell.size / props.data.parent_aggregate_size) * 100;
  if (pct < 0.1) return "<0.1%";
  return `${pct.toFixed(1)}%`;
}
</script>

<template>
  <div class="treemap-wrap">
    <svg
      v-if="data && cells.length"
      class="treemap-svg"
      :viewBox="`0 0 ${WIDTH} ${HEIGHT}`"
      preserveAspectRatio="xMidYMid meet"
    >
      <g>
        <g
          v-for="cell in cells"
          :key="cell.id"
          :class="{ cell: true, 'cell-dir': cell.is_dir, 'cell-active': hoveredId === cell.id }"
          @mouseenter="hoveredId = cell.id"
          @mouseleave="hoveredId = null"
          @click="onClick(cell)"
        >
          <rect
            :x="cell.x"
            :y="cell.y"
            :width="cell.w"
            :height="cell.h"
            :fill="cell.fill"
            class="cell-rect"
          />
          <text
            v-if="cell.showLabel"
            :x="cell.x + 6"
            :y="cell.y + cell.labelFontSize + 4"
            :font-size="cell.labelFontSize"
            class="cell-name"
          >
            {{ cell.name.length > 24 ? cell.name.slice(0, 23) + "…" : cell.name }}
          </text>
          <text
            v-if="cell.showLabel && cell.h >= 40"
            :x="cell.x + 6"
            :y="cell.y + cell.labelFontSize * 2 + 8"
            :font-size="Math.max(cell.labelFontSize - 2, 9)"
            class="cell-size"
          >
            {{ formatBytes(cell.size) }}
          </text>
          <text
            v-if="cell.showLabel && cell.h >= 60 && cell.scoreLabel"
            :x="cell.x + 6"
            :y="cell.y + cell.labelFontSize * 3 + 12"
            :font-size="9"
            class="cell-score"
          >
            {{ cell.scoreLabel }}
          </text>
          <title>
            {{ cell.name }} · {{ formatBytes(cell.size) }} · {{ percentOf(cell) }}
          </title>
        </g>
      </g>
    </svg>
    <div v-else class="treemap-empty">Treemap için yeterli veri yok.</div>

    <div v-if="hoveredCell" class="treemap-readout mono">
      <span class="readout-name">{{ hoveredCell.name }}</span>
      <span class="readout-size">{{ formatBytes(hoveredCell.size) }}</span>
      <span class="readout-pct">{{ percentOf(hoveredCell) }}</span>
      <span v-if="hoveredCell.scoreLabel" class="readout-score">
        {{ hoveredCell.scoreLabel }}
      </span>
    </div>
  </div>
</template>

<style scoped>
.treemap-wrap {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 12px 0 4px;
}

.treemap-svg {
  width: 100%;
  height: auto;
  background: #0b0d10;
  border-radius: 10px;
  user-select: none;
}

.cell-rect {
  stroke: #0b0d10;
  stroke-width: 1.5;
  transition: opacity 0.15s, filter 0.15s;
  opacity: 0.85;
}

.cell-dir .cell-rect {
  cursor: pointer;
}

.cell-dir:hover .cell-rect,
.cell-active .cell-rect {
  opacity: 1;
  filter: brightness(1.25)
    drop-shadow(0 0 6px rgba(36, 200, 219, 0.45));
}

.cell-name {
  fill: #f5f5f5;
  font-family: ui-monospace, monospace;
  pointer-events: none;
  text-shadow: 0 0 4px rgba(0, 0, 0, 0.7);
}

.cell-size {
  fill: #d1d5db;
  font-family: ui-monospace, monospace;
  pointer-events: none;
  text-shadow: 0 0 4px rgba(0, 0, 0, 0.7);
}

.cell-score {
  fill: #fcd34d;
  font-family: ui-monospace, monospace;
  letter-spacing: 0.04em;
  pointer-events: none;
  text-shadow: 0 0 4px rgba(0, 0, 0, 0.7);
}

.treemap-empty {
  padding: 60px 0;
  color: var(--muted);
  font-size: 13px;
  text-align: center;
}

.treemap-readout {
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
