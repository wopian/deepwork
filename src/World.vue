<script setup lang="ts">
import { onMounted, onBeforeUnmount, ref, watch } from "vue";
import { Application, Graphics, Text, Container } from "pixi.js";
import { state, materials } from "./game";
import { WasteParticles } from "./waste";
import profiles from "../content/sites.json";
import { preferences } from "./preferences";
const host = ref<HTMLDivElement>();
let app: Application | undefined;
let world: Container;
let terrain: Graphics;
let actors: Graphics;
let t = 0;
const wasteParticles = new WasteParticles(600);
let lastWaste = 0;
let lastSite = 0;
let zoom = 1;
let offsetY = 0;
let dragY: number | null = null;
let dragX: number | null = null;
let offsetX = 0;
let follow = false;
let stopWatch: () => void = () => {};

function rect(
  g: Graphics,
  x: number,
  y: number,
  w: number,
  h: number,
  c: number,
) {
  g.rect(Math.round(x), Math.round(y), w, h).fill(c);
}
function draw() {
  if (!app) return;
  terrain.clear();
  const g = state.value;
  const W = 1100;
  const first = Math.max(
    0,
    Math.floor(-offsetY / ((app.screen.width / 1100) * zoom) / 7) - 30,
  );
  const last = first + 130;
  rect(terrain, 0, 190, W, (last + 5) * 7, 0x806044);
  rect(terrain, 0, 188, W, 8, 0x6b8f47);
  rect(terrain, 0, 196, W, 12, 0xd8bc7d);
  for (let x = 0; x < 64; x++) {
    const h = g?.heights[x] ?? Math.floor(12 * Math.sin((x / 64) * Math.PI));
    for (let y = first; y < last; y++) {
      const mask = g?.terrain.chunks[Math.floor(y / 64)];
      const index = (y % 64) * 64 + x;
      const open = g ? !!(mask && mask[index >> 3] & (1 << index % 8)) : y < h;
      if (open) {
        rect(terrain, 235 + x * 7, 208 + y * 7, 7, 7, 0x101820);
        continue;
      }
      const hash = BigInt.asUintN(
        64,
        (BigInt(g?.seed ?? 73429) +
          BigInt(Math.floor(x / 4)) * 374761393n +
          BigInt(Math.floor(y / 3)) * 668265263n) *
          1274126177n,
      );
      const tier =
        y * 2 < 100
          ? 0
          : y * 2 < 300
            ? 1
            : y * 2 < 700
              ? 2
              : y * 2 < 1500
                ? 3
                : y * 2 < 3000
                  ? 4
                  : 5;
      const pool = materials.filter((m) => m.tier <= tier);
      pool.push(
        ...materials.filter(
          (m) =>
            m.tier <= tier && profiles[g?.profile ?? 0].focus.includes(m.id),
        ),
      );
      const id =
        x >= 30 && x <= 34 && y % 24 < 3
          ? [3, 5, 6][Math.floor(y / 24) % 3]
          : hash % 100n < 55n
            ? y < 4
              ? 0
              : 1
            : pool[Number((hash >> 8n) % BigInt(pool.length))].id;
      if (id > 1) {
        const m = materials[id];
        rect(
          terrain,
          235 + x * 7,
          208 + y * 7,
          6,
          6,
          parseInt(m.color.slice(1), 16),
        );
      } else if (hash % 9n === 0n) {
        rect(terrain, 235 + x * 7, 208 + y * 7, 3, 2, 0x9d7751);
      }
    }
  }
  // Timber frames, conveyor and processing districts.
  for (let x = 35; x < 180; x += 55) {
    rect(terrain, x, 148, 46, 40, 0xa67548);
    rect(terrain, x - 4, 142, 54, 8, 0xe8dfc8);
    rect(terrain, x + 14, 165, 13, 23, 0x101820);
  }
  if (!g || g.levels.conveyor) rect(terrain, 213, 170, 478, 8, 0x8c9ba5);
  for (let x = 216; x < 690; x += 28) {
    rect(terrain, x, 178, 4, 22, 0xa67548);
  }
  for (
    let i = 0;
    i <
    (g
      ? ["furnace", "chemical", "electrolytic"].filter((k) => g.levels[k])
          .length
      : 3);
    i++
  ) {
    let x = 715 + i * 68;
    rect(terrain, x, 136, 48, 51, 0x8c9ba5);
    rect(terrain, x + 8, 143, 30, 34, 0x101820);
    rect(terrain, x + 14, 154, 18, 19, 0xe5a34d);
    rect(terrain, x + 29, 111, 10, 25, 0x8c9ba5);
  }
  const storedWaste = g
    ? Object.values(g.tailings).reduce((a, b) => a + b, 0) + g.slag + g.depleted
    : 0;
  const waste = Math.min(64, 12 + storedWaste / 3000);
  for (let i = 0; i < 18; i++) {
    let h = Math.max(0, waste - Math.abs(i - 9) * 4);
    rect(terrain, 939 + i * 7, 190 - h, 7, h, 0x8c9ba5);
  }
}
onMounted(async () => {
  app = new Application();
  await app.init({
    resizeTo: host.value!,
    background: 0x101820,
    antialias: false,
    preference: "webgl",
    resolution: Math.min(devicePixelRatio, 2),
    autoDensity: true,
  });
  if (!host.value) {
    app.destroy(true);
    return;
  }
  host.value.appendChild(app.canvas);
  world = new Container();
  terrain = new Graphics();
  actors = new Graphics();
  world.addChild(terrain, actors);
  app.stage.addChild(world);
  for (const [label, x] of [
    ["CAMP", 55],
    ["EXCAVATION", 370],
    ["PROCESSING", 715],
    ["SPOIL", 960],
  ] as const) {
    const text = new Text({
      text: label,
      style: {
        fontFamily: "monospace",
        fontSize: 11,
        fill: 0xe8dfc8,
        letterSpacing: 3,
      },
    });
    text.position.set(x, 83);
    world.addChild(text);
  }
  draw();
  stopWatch = watch(state, draw);
  app.ticker.add((ticker) => {
    if (!app) return;
    t += preferences.reducedMotion ? 0 : ticker.deltaTime;
    world.scale.set((app.screen.width / 1100) * zoom);
    if (follow) {
      offsetY =
        80 -
        ((Math.max(...(state.value?.heights ?? [0])) * 7 * app.screen.width) /
          1100) *
          zoom;
    }
    world.y = offsetY;
    world.x = offsetX;
    actors.clear();
    const g = state.value;
    if (g) {
      if (lastSite !== g.site) {
        lastSite = g.site;
        lastWaste = g.lifetime_waste;
        wasteParticles.items.length = 0;
      }
      if (!preferences.reducedMotion)
        wasteParticles.emit(
          Math.ceil(Math.max(0, g.lifetime_waste - lastWaste) / 50),
        );
      lastWaste = g.lifetime_waste;
    }
    wasteParticles.step(ticker.deltaTime, 188);
    for (const particle of wasteParticles.items) {
      rect(actors, particle.x, particle.y, 3, 3, 0x8c9ba5);
    }
    for (let i = 0; i < Math.min(g?.workers ?? 9, 100); i++) {
      let x = 240 + ((i * 43 + t * (i % 2 ? 0.5 : -0.35) + 44800) % 430);
      const col = Math.max(0, Math.min(63, Math.floor((x - 235) / 7)));
      let y =
        205 +
        (g?.heights[col] ?? Math.floor(12 * Math.sin((col / 64) * Math.PI))) *
          7;
      rect(actors, x, y - 10, 5, 5, 0xe8dfc8);
      rect(actors, x - 1, y - 5, 7, 5, 0xe5a34d);
      rect(actors, x + 5, y - 8, 5, 2, 0x8c9ba5);
    }
    for (const cargo of g?.shipments ?? []) {
      const progress = 1 - cargo.remaining / cargo.duration;
      const x = 235 + 32 * 7 + progress * 240;
      const y = 208 + (((1 - progress) * cargo.depth) / 2) * 7;
      rect(
        actors,
        x,
        y,
        9,
        5,
        parseInt(materials[cargo.material].color.slice(1), 16),
      );
      if (cargo.mode === "minecart" || cargo.mode === "train") {
        rect(actors, x - 2, y + 5, 13, 4, 0x8c9ba5);
      }
    }
  });
});
onBeforeUnmount(() => {
  stopWatch();
  app?.destroy(true, { children: true });
});
function wheel(e: WheelEvent) {
  zoom = Math.max(0.7, Math.min(2.5, zoom - e.deltaY * 0.001));
}
</script>
<template>
  <div class="world-wrap">
    <div
      ref="host"
      class="world"
      @wheel.prevent="wheel"
      @pointerdown="
        (e: PointerEvent) => {
          follow = false;
          dragY = e.clientY;
          dragX = e.clientX;
          (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
        }
      "
      @pointermove="
        (e: PointerEvent) => {
          if (dragY !== null) {
            offsetY += e.clientY - dragY;
            offsetX += e.clientX - (dragX ?? e.clientX);
            dragY = e.clientY;
            dragX = e.clientX;
            draw();
          }
        }
      "
      @pointerup="dragY = null"
      @pointercancel="dragY = null"
    />
    <div class="world-tools">
      <button @click="zoom = Math.min(2.5, zoom + 0.2)" aria-label="Zoom in">
        ＋</button
      ><button @click="zoom = Math.max(0.7, zoom - 0.2)" aria-label="Zoom out">
        −</button
      ><button
        @click="
          () => {
            offsetY = 0;
            offsetX = 0;
            zoom = 1;
            follow = false;
            draw();
          }
        "
      >
        Surface ↑</button
      ><button
        @click="
          () => {
            follow = !follow;
            draw();
          }
        "
      >
        Follow depth
      </button>
    </div>
    <div class="world-caption">
      {{ state ? "LIVE OPERATION" : "ILLUSTRATIVE PREVIEW" }}
      <span>SCROLL TO ZOOM · DRAG TO EXPLORE</span>
    </div>
  </div>
</template>
