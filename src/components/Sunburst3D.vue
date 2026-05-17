<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<!--
  Sunburst3D — 3D katmanlı döndürülebilir sunburst (Bölüm 9.1 mode 5).
  Three.js + OrbitControls + ExtrudeGeometry.

  Her depth seviyesi Y ekseninde ayrı bir katman:
    * Center disk (depth 0)        → y=0,  height=4
    * Inner ring (depth 1)         → y=4,  height=20
    * Outer ring (depth 2)         → y=24, height=20

  Mouse drag → her yön döndürme (OrbitControls).
  Mouse wheel → zoom.
  Hover    → wedge brightens.
  Click    → @drill event'i yansır (parent drilldown).

  Tableau 10 palet ile 2D sunburst görsel sürekliliği korunur.
-->
<script setup lang="ts">
import {
  onBeforeUnmount,
  onMounted,
  ref,
  shallowRef,
  watch,
} from "vue";
import * as THREE from "three";
import { OrbitControls } from "three/addons/controls/OrbitControls.js";

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

const props = defineProps<{ data: WindowResult | null }>();
const emit = defineEmits<{ (e: "drill", node: TreeNode): void }>();

const canvasRef = ref<HTMLCanvasElement | null>(null);
const hoveredLabel = ref<string>("");

// Tableau 10 — 2D sunburst ile aynı palet.
const PALETTE = [
  0x4e79a7, 0xf28e2c, 0xe15759, 0x76b7b2, 0x59a14f, 0xedc949, 0xaf7aa1,
  0xff9da7, 0x9c755f, 0xbab0ab, 0x5b8ff9, 0x5ad8a6,
];

const INNER_R = 32;
const RING_THICKNESS = 70;
const CENTER_HEIGHT = 4;
const RING_HEIGHT = 22;
const GAP_RAD = 0.012;

interface SceneState {
  renderer: THREE.WebGLRenderer;
  scene: THREE.Scene;
  camera: THREE.PerspectiveCamera;
  controls: OrbitControls;
  raycaster: THREE.Raycaster;
  pointer: THREE.Vector2;
  wedges: THREE.Mesh[]; // tıklanabilir wedge mesh listesi
  hovered: THREE.Mesh | null;
  animation: number | null;
}
const sceneState = shallowRef<SceneState | null>(null);

function buildWedgeShape(
  a1: number,
  a2: number,
  innerR: number,
  outerR: number,
): THREE.Shape {
  const shape = new THREE.Shape();
  shape.absarc(0, 0, outerR, a1, a2, false);
  shape.absarc(0, 0, innerR, a2, a1, true);
  return shape;
}

function colorFor(index: number): number {
  return PALETTE[index % PALETTE.length];
}

function buildScene(canvas: HTMLCanvasElement): SceneState {
  const w = canvas.clientWidth || 480;
  const h = canvas.clientHeight || 360;

  const renderer = new THREE.WebGLRenderer({
    canvas,
    antialias: true,
    alpha: true,
  });
  renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
  renderer.setSize(w, h, false);

  const scene = new THREE.Scene();
  scene.background = null;

  const camera = new THREE.PerspectiveCamera(50, w / h, 1, 2000);
  camera.position.set(160, 180, 220);
  camera.lookAt(0, 0, 0);

  // Aydınlatma: ambient + iki directional ile depth hissi
  const ambient = new THREE.AmbientLight(0xffffff, 0.55);
  scene.add(ambient);
  const dir1 = new THREE.DirectionalLight(0xffffff, 0.85);
  dir1.position.set(120, 200, 80);
  scene.add(dir1);
  const dir2 = new THREE.DirectionalLight(0xffffff, 0.35);
  dir2.position.set(-80, 100, -120);
  scene.add(dir2);

  const controls = new OrbitControls(camera, canvas);
  controls.enableDamping = true;
  controls.dampingFactor = 0.1;
  controls.minDistance = 80;
  controls.maxDistance = 600;
  controls.target.set(0, 30, 0);
  controls.update();

  // Center disk (parent label görsel anchor)
  const centerGeo = new THREE.CylinderGeometry(
    INNER_R - 2,
    INNER_R - 2,
    CENTER_HEIGHT,
    48,
  );
  const centerMat = new THREE.MeshStandardMaterial({
    color: 0x2a2c34,
    roughness: 0.6,
    metalness: 0.1,
  });
  const center = new THREE.Mesh(centerGeo, centerMat);
  center.position.y = CENTER_HEIGHT / 2;
  scene.add(center);

  return {
    renderer,
    scene,
    camera,
    controls,
    raycaster: new THREE.Raycaster(),
    pointer: new THREE.Vector2(),
    wedges: [],
    hovered: null,
    animation: null,
  };
}

function clearWedges(state: SceneState) {
  for (const m of state.wedges) {
    state.scene.remove(m);
    m.geometry.dispose();
    if (Array.isArray(m.material)) {
      m.material.forEach((mat) => mat.dispose());
    } else {
      m.material.dispose();
    }
  }
  state.wedges = [];
}

function rebuildWedges(state: SceneState, data: WindowResult) {
  clearWedges(state);
  const total = data.parent_aggregate_size;
  if (total <= 0) return;

  const yBase = CENTER_HEIGHT + 1; // katman 1 başlangıcı
  let angle = 0;
  const n = data.nodes.length;
  for (let i = 0; i < n; i++) {
    const node = data.nodes[i];
    const ratio = node.aggregate_size / total;
    if (ratio <= 0) continue;
    const a1 = angle;
    const a2 = angle + ratio * Math.PI * 2;
    angle = a2;
    const a1g = a1 + GAP_RAD / 2;
    const a2g = a2 - GAP_RAD / 2;
    if (a2g <= a1g) continue;

    const shape = buildWedgeShape(a1g, a2g, INNER_R, INNER_R + RING_THICKNESS);
    const geo = new THREE.ExtrudeGeometry(shape, {
      depth: RING_HEIGHT,
      bevelEnabled: true,
      bevelThickness: 1.5,
      bevelSize: 1,
      bevelSegments: 2,
      curveSegments: Math.max(8, Math.floor((a2g - a1g) * 24)),
    });
    // ExtrudeGeometry XY düzleminde — Y eksenini "yukarı" yapacak şekilde
    // rotateX(-PI/2) ile döndür, sonra Y'de yBase'e taşı.
    geo.rotateX(-Math.PI / 2);

    const mat = new THREE.MeshStandardMaterial({
      color: colorFor(i),
      roughness: 0.45,
      metalness: 0.15,
      emissive: 0x000000,
    });
    const mesh = new THREE.Mesh(geo, mat);
    mesh.position.y = yBase;
    mesh.userData = {
      kind: "wedge",
      node,
      baseColor: colorFor(i),
      baseY: yBase,
    };
    state.scene.add(mesh);
    state.wedges.push(mesh);
  }
}

function onResize(state: SceneState, canvas: HTMLCanvasElement) {
  const w = canvas.clientWidth;
  const h = canvas.clientHeight;
  if (w === 0 || h === 0) return;
  state.camera.aspect = w / h;
  state.camera.updateProjectionMatrix();
  state.renderer.setSize(w, h, false);
}

function setHover(state: SceneState, next: THREE.Mesh | null) {
  if (state.hovered === next) return;
  if (state.hovered) {
    const mat = state.hovered.material as THREE.MeshStandardMaterial;
    mat.emissive.setHex(0x000000);
    state.hovered.position.y = state.hovered.userData.baseY;
  }
  state.hovered = next;
  if (next) {
    const mat = next.material as THREE.MeshStandardMaterial;
    mat.emissive.setHex(0x222222);
    next.position.y = (next.userData.baseY as number) + 3;
    const node = next.userData.node as TreeNode;
    hoveredLabel.value = `${node.is_dir ? "📁" : "📄"} ${node.name} · ${formatBytes(node.aggregate_size)}`;
  } else {
    hoveredLabel.value = "";
  }
}

function onPointerMove(state: SceneState, canvas: HTMLCanvasElement, e: PointerEvent) {
  const rect = canvas.getBoundingClientRect();
  state.pointer.x = ((e.clientX - rect.left) / rect.width) * 2 - 1;
  state.pointer.y = -((e.clientY - rect.top) / rect.height) * 2 + 1;
  state.raycaster.setFromCamera(state.pointer, state.camera);
  const hits = state.raycaster.intersectObjects(state.wedges, false);
  if (hits.length > 0) {
    setHover(state, hits[0].object as THREE.Mesh);
  } else {
    setHover(state, null);
  }
}

function onPointerDown(state: SceneState, e: PointerEvent) {
  if (e.button !== 0) return;
  if (state.hovered) {
    const node = state.hovered.userData.node as TreeNode;
    emit("drill", node);
  }
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

let resizeObs: ResizeObserver | null = null;
let pointerMoveHandler: ((e: PointerEvent) => void) | null = null;
let pointerDownHandler: ((e: PointerEvent) => void) | null = null;

onMounted(() => {
  if (!canvasRef.value) return;
  const state = buildScene(canvasRef.value);
  sceneState.value = state;

  if (props.data) rebuildWedges(state, props.data);

  // Animation loop
  const animate = () => {
    state.controls.update();
    state.renderer.render(state.scene, state.camera);
    state.animation = requestAnimationFrame(animate);
  };
  state.animation = requestAnimationFrame(animate);

  // Resize observer
  resizeObs = new ResizeObserver(() => {
    if (canvasRef.value) onResize(state, canvasRef.value);
  });
  resizeObs.observe(canvasRef.value);

  // Pointer events
  pointerMoveHandler = (e) => onPointerMove(state, canvasRef.value!, e);
  pointerDownHandler = (e) => onPointerDown(state, e);
  canvasRef.value.addEventListener("pointermove", pointerMoveHandler);
  canvasRef.value.addEventListener("pointerdown", pointerDownHandler);
});

onBeforeUnmount(() => {
  const state = sceneState.value;
  if (state) {
    if (state.animation !== null) cancelAnimationFrame(state.animation);
    clearWedges(state);
    state.controls.dispose();
    state.renderer.dispose();
  }
  if (resizeObs) resizeObs.disconnect();
  if (canvasRef.value && pointerMoveHandler) {
    canvasRef.value.removeEventListener("pointermove", pointerMoveHandler);
  }
  if (canvasRef.value && pointerDownHandler) {
    canvasRef.value.removeEventListener("pointerdown", pointerDownHandler);
  }
});

watch(
  () => props.data,
  (next) => {
    const state = sceneState.value;
    if (state && next) rebuildWedges(state, next);
  },
);
</script>

<template>
  <div class="sun3d-wrap">
    <canvas ref="canvasRef" class="sun3d-canvas" />
    <div class="sun3d-hud" v-if="hoveredLabel">{{ hoveredLabel }}</div>
    <div class="sun3d-hint">🖱 sürükle = döndür · tekerlek = zoom · tık = içeri</div>
  </div>
</template>

<style scoped>
.sun3d-wrap {
  position: relative;
  width: 100%;
  height: 480px;
  border-radius: 12px;
  overflow: hidden;
  background: linear-gradient(
    180deg,
    var(--bg) 0%,
    color-mix(in srgb, var(--bg) 70%, transparent) 100%
  );
  border: 1px solid var(--border);
}

.sun3d-canvas {
  width: 100%;
  height: 100%;
  display: block;
  cursor: grab;
  touch-action: none;
}

.sun3d-canvas:active {
  cursor: grabbing;
}

.sun3d-hud {
  position: absolute;
  top: 10px;
  left: 10px;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 6px 10px;
  font-size: 12px;
  color: var(--fg);
  font-family: ui-monospace, "Cascadia Code", monospace;
  box-shadow: 0 4px 10px var(--shadow);
  max-width: 80%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  pointer-events: none;
}

.sun3d-hint {
  position: absolute;
  bottom: 8px;
  right: 12px;
  font-size: 10px;
  color: var(--muted);
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 999px;
  padding: 3px 9px;
  pointer-events: none;
  opacity: 0.85;
}
</style>
