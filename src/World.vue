<script setup lang="ts">
import { onMounted, onBeforeUnmount, ref, watch } from "vue";
import { Application, Graphics, Text, Container } from "pixi.js";
// Pixi shader/uniform polyfills preserve the native CSP without eval.
import "pixi.js/unsafe-eval";
import { state, materials, terrainEpoch, format } from "./game";
import { WasteParticles } from "./waste";
import profiles from "../content/sites.json";
import { routePosition, cargoPosition } from "./routes";
import { preferences, productionAudio } from "./preferences";
const host = ref<HTMLDivElement>();
let app: Application | undefined;
let world: Container;
let terrain: Graphics;
let actors: Graphics;
let wasteLabel: Text;
let t = 0;
let telemetryTime = 0;
let telemetryFrames = 0;
const wasteParticles = new WasteParticles(600);
let drawnKey = "";
let lastWaste = 0;
let lastSite = 0;
let zoom = 1;
let offsetY = 0;
let dragY: number | null = null;
let dragX: number | null = null;
let offsetX = 0;
let follow = false;
let followCrew = false;
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
  const g = state.value;
  const W = 1100;
  const first = Math.max(
    0,
    Math.floor(-offsetY / ((app.screen.width / 1100) * zoom) / 7) - 30,
  );
  const last =
    first +
    Math.ceil(app.screen.height / ((app.screen.width / 1100) * zoom) / 7) +
    32;
  const stored = g
    ? Object.values(g.tailings).reduce((a, b) => a + b, 0) + g.slag + g.depleted
    : 0;
  const key = [
    terrainEpoch.value,
    g?.site,
    g?.seed,
    g?.profile,
    g?.terrain.revision,
    g?.support_rows,
    g?.housing,
    first,
    last,
    Math.floor(stored / 3000),
    Math.floor((g?.lifetime_waste ?? 0) / 1000),
    JSON.stringify(g?.levels),
  ].join(":");
  if (key === drawnKey) return;
  drawnKey = key;
  terrain.clear();
  const pools = Array.from({ length: 6 }, (_, tier) => [
    ...materials.filter((m) => m.tier <= tier),
    ...materials.filter(
      (m) => m.tier <= tier && profiles[g?.profile ?? 0].focus.includes(m.id),
    ),
  ]);
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
      const pool = pools[tier];
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
        // Shape marks distinguish deposits without relying only on hue.
        if (id % 3 === 0)
          rect(terrain, 235 + x * 7 + 2, 208 + y * 7, 1, 6, 0x101820);
        else if (id % 3 === 1)
          rect(terrain, 235 + x * 7 + 2, 208 + y * 7 + 2, 2, 2, 0x101820);
        else rect(terrain, 235 + x * 7, 208 + y * 7 + 3, 6, 1, 0x101820);
      } else if (hash % 9n === 0n) {
        rect(terrain, 235 + x * 7, 208 + y * 7, 3, 2, 0x9d7751);
      }
    }
  }
  if (g?.levels.supports) {
    for (
      let y = Math.max(first, 150);
      y < Math.min(last, g.support_rows);
      y++
    ) {
      if (y % 12 !== 0) continue;
      for (let x = 31; x <= 33; x++) {
        const index = (y % 64) * 64 + x,
          mask = g.terrain.chunks[Math.floor(y / 64)];
        if (mask && mask[index >> 3] & (1 << index % 8)) {
          rect(terrain, 235 + x * 7, 208 + y * 7, 7, 1, 0xa67548);
          rect(terrain, 235 + x * 7, 208 + y * 7, 1, 7, 0xa67548);
        }
      }
    }
  }
  // Fixed district slots grow upward, keeping routes and touch camera targets stable.
  const levels = g?.levels ?? {
    conveyor: 10,
    furnace: 1,
    chemical: 1,
    electrolytic: 1,
  };
  const housing = g?.housing ?? 8;
  for (let i = 0; i < 3; i++) {
    const x = 35 + i * 55;
    const floors = Math.min(4, 1 + Math.floor(Math.max(0, housing - 8) / 12));
    for (let floor = 0; floor < floors; floor++) {
      const y = 158 - floor * 15;
      rect(terrain, x, y, 46, 30, 0xa67548);
      rect(terrain, x - 3, y - 4, 52, 4, 0xe8dfc8);
      for (let window = 0; window < 3; window++)
        rect(terrain, x + 5 + window * 14, y + 5, 6, 6, 0x101820);
    }
    rect(terrain, x + 17, 172, 12, 16, 0x101820);
  }
  if (levels.conveyor) {
    rect(terrain, 213, 170, 478, 8, 0x8c9ba5);
    for (let x = 216; x < 690; x += 28) {
      rect(terrain, x, 178, 4, 22, 0xa67548);
      rect(terrain, x + 4, 173, 4, 3, 0x101820);
    }
    const lanes = Math.min(5, Math.floor(levels.conveyor / 10));
    for (let lane = 0; lane < lanes; lane++)
      rect(terrain, 213, 167 - lane * 3, 478, 1, 0xe5a34d);
  }
  if (levels.shaft) {
    rect(terrain, 444, 130, 5, 70, 0x8c9ba5);
    rect(terrain, 477, 130, 5, 70, 0x8c9ba5);
    rect(terrain, 440, 126, 46, 6, 0x8c9ba5);
    for (let y = 137; y < 164; y += 9) rect(terrain, 449, y, 28, 2, 0xa67548);
    rect(terrain, 458, 120, 11, 11, 0xe5a34d);
    rect(terrain, 462, 130, 2, 61, 0xe8dfc8);
    for (let tier = 0; tier < Math.floor(levels.shaft / 10); tier++)
      rect(terrain, 487 + tier * 4, 149, 2, 40, 0x8c9ba5);
  }
  if (levels.minecart || levels.train) {
    rect(terrain, 685, 185, 33, 2, 0x8c9ba5);
    rect(terrain, 688, 175, 24, 9, 0xa67548);
    rect(terrain, 691, 184, 4, 4, 0xe8dfc8);
    rect(terrain, 706, 184, 4, 4, 0xe8dfc8);
    if (levels.train) {
      rect(terrain, 688, 165, 8, 10, 0x8c9ba5);
      rect(terrain, 702, 169, 10, 6, 0x8c9ba5);
    }
  }
  // Shared halls expose installed modules; badges mark each ten-level tier.
  const halls = [
    {
      x: 724,
      ids: ["sorter", "furnace", "steelworks", "recovery"],
      kind: "heat",
    },
    {
      x: 791,
      ids: ["chemical", "electrolytic", "power", "pump"],
      kind: "chemical",
    },
    {
      x: 858,
      ids: ["trace", "manufacturing", "survey", "ventilation"],
      kind: "precision",
    },
  ];
  for (const hall of halls) {
    const modules = hall.ids.filter((id) => levels[id]);
    if (!modules.length && hall.kind !== "heat") continue;
    const tier = Math.min(
      5,
      Math.floor(Math.max(0, ...modules.map((id) => levels[id]!)) / 10),
    );
    const height = 34 + modules.length * 6 + tier * 3;
    const x = hall.x,
      y = 188 - height;
    rect(terrain, x, y, 51, height, 0x8c9ba5);
    rect(terrain, x + 4, y + 4, 43, height - 8, 0x101820);
    if (hall.kind === "heat") {
      rect(terrain, x + 9, 168, 15, 15, levels.furnace ? 0xe5a34d : 0xa67548);
      rect(terrain, x + 30, y - 20, 8, 24, 0x8c9ba5);
      if (levels.steelworks) rect(terrain, x + 29, 159, 14, 23, 0xe5a34d);
    } else if (hall.kind === "chemical") {
      for (let tank = 0; tank < 3; tank++) {
        rect(terrain, x + 8 + tank * 13, y + 14, 9, height - 21, 0xe8dfc8);
        rect(terrain, x + 11 + tank * 13, y + 7, 3, 10, 0x8c9ba5);
      }
      rect(terrain, x + 11, y + 8, 29, 2, 0xe5a34d);
    } else {
      for (let column = 0; column < 3; column++)
        for (let row = 0; row < 3; row++)
          rect(terrain, x + 9 + column * 12, y + 12 + row * 10, 6, 6, 0xe5a34d);
    }
    for (let module = 0; module < modules.length; module++)
      rect(terrain, x + 7 + module * 10, y + 3, 6, 3, 0xe8dfc8);
    for (let badge = 0; badge < tier; badge++)
      rect(terrain, x + 4 + badge * 9, 190, 5, 3, 0xe5a34d);
  }
  if (levels.reclaimer || levels.slagcrusher) {
    rect(terrain, 1009, 149, 42, 39, 0x8c9ba5);
    rect(terrain, 1015, 155, 30, 25, 0x101820);
    rect(terrain, 1022, 163, 16, 8, 0xe5a34d);
    rect(terrain, 992, 179, 22, 4, 0xa67548);
  }
  const storedWaste = g
    ? Object.values(g.tailings).reduce((a, b) => a + b, 0) + g.slag + g.depleted
    : 0;
  wasteLabel.text = `LIFETIME ${format((g?.lifetime_waste ?? 0) / 1000)} units`;
  // Older spoil uses a bounded stack of coarse bands, independent of resource ledgers.
  const strata = Math.min(12, Math.floor(Math.log2(1 + storedWaste / 1000)));
  for (let band = 0; band < strata; band++)
    rect(terrain, 925, 208 + band * 7, 150, 6, band % 2 ? 0x806044 : 0x8c9ba5);
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
  wasteLabel = new Text({
    text: "",
    style: { fontFamily: "monospace", fontSize: 9, fill: 0xe8dfc8 },
  });
  wasteLabel.position.set(925, 310);
  world.addChild(wasteLabel);
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
    if (followCrew && state.value?.removed.length) {
      const cell = state.value.removed[state.value.removed.length - 1]!;
      offsetX =
        app.screen.width / 2 -
        (((235 + cell.x * 7) * app.screen.width) / 1100) * zoom;
      offsetY =
        app.screen.height / 2 -
        (((208 + cell.y * 7) * app.screen.width) / 1100) * zoom;
    } else if (follow) {
      offsetY =
        80 -
        ((Math.max(...(state.value?.heights ?? [0])) * 7 * app.screen.width) /
          1100) *
          zoom;
    }
    draw();
    const low =
      preferences.quality === "low" ||
      (preferences.quality === "auto" && innerWidth < 700);
    wasteParticles.limit = low ? 600 : 2000;
    app.ticker.maxFPS = low ? 30 : 60;
    world.y = offsetY;
    world.x = offsetX;
    actors.clear();
    const g = state.value;
    productionAudio(g?.stages.reduce((sum, s) => sum + s.rate, 0) ?? 0);
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
    const crew = g?.crew ?? { diggers: 6, haulers: 3 };
    let shown = 0;
    const workerBudget = low ? 100 : 250;
    for (const [role, count] of Object.entries(crew)) {
      for (let i = 0; i < count && shown < workerBudget; i++, shown++) {
        let x = 250 + i * 8,
          y = 185;
        if (role === "diggers") {
          const cell =
            g?.removed[
              Math.max(
                0,
                g.removed.length - 1 - (i % Math.max(1, g.removed.length)),
              )
            ];
          if (cell) {
            x = 235 + cell.x * 7;
            y = 208 + cell.y * 7 + 7;
          }
        } else if (role === "haulers") {
          const cargo = g?.shipments[i % Math.max(1, g.shipments.length)];
          if (cargo?.path.length) {
            const points: [number, number][] = cargo.path.map(([px, py]) => [
              235 + px * 7,
              208 + py * 7 + 7,
            ]);
            [x, y] = routePosition(
              points,
              1 - cargo.remaining / cargo.duration,
            );
          }
        } else {
          x =
            (
              {
                operators: 735,
                engineers: 690,
                prospectors: 225,
                reclaimers: 980,
              } as Record<string, number>
            )[role] ?? 100;
          x += (i % 8) * 8;
        }
        rect(actors, x, y - 7, 4, 3, 0xe8dfc8);
        rect(actors, x, y - 4, 5, 4, role === "diggers" ? 0xe5a34d : 0x8c9ba5);
        if (role === "diggers")
          rect(
            actors,
            x + 4,
            y - 5 + (Math.floor(t / 12 + i) % 2),
            3,
            1,
            0x8c9ba5,
          );
      }
    }
    telemetryFrames++;
    if (host.value && performance.now() - telemetryTime > 1000) {
      const now = performance.now();
      host.value.dataset.fps = String(
        Math.round((telemetryFrames * 1000) / (now - telemetryTime)),
      );
      telemetryFrames = 0;
      telemetryTime = now;
      host.value.dataset.workers = String(shown);
      host.value.dataset.particles = String(wasteParticles.items.length);
    }
    for (const cargo of g?.shipments ?? []) {
      const progress = 1 - cargo.remaining / cargo.duration;
      const points: [number, number][] = cargo.path?.length
        ? cargo.path.map(([x, y]) => [235 + x * 7, 208 + y * 7])
        : [[459, 208 + (cargo.depth / 2) * 7]];
      points.push([points[points.length - 1]![0], 185], [735, 185]);
      const leg = cargoPosition(
        cargo.legs ?? [],
        cargo.duration - cargo.remaining,
      );
      const [x, y] = leg
        ? [235 + leg.point[0] * 7, 208 + leg.point[1] * 7]
        : routePosition(points, progress);
      const mode = leg?.mode ?? cargo.mode;
      rect(
        actors,
        x,
        y,
        9,
        5,
        parseInt(materials[cargo.material].color.slice(1), 16),
      );
      if (mode === "minecart" || mode === "train") {
        rect(actors, x - 2, y + 5, 13, 4, 0x8c9ba5);
      } else if (mode === "lift") {
        rect(actors, x - 2, y - 3, 1, 12, 0x8c9ba5);
        rect(actors, x + 10, y - 3, 1, 12, 0x8c9ba5);
        rect(actors, x - 2, y + 8, 13, 1, 0x8c9ba5);
      }
    }
  });
});
onBeforeUnmount(() => {
  productionAudio(0);
  stopWatch();
  app?.destroy(true, { children: true });
});
function focusDistrict(x: number) {
  if (!app) return;
  follow = false;
  followCrew = false;
  offsetY = 0;
  offsetX = app.screen.width / 2 - ((x * app.screen.width) / 1100) * zoom;
  draw();
}
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
          followCrew = false;
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
            followCrew = false;
            draw();
          }
        "
      >
        Surface ↑</button
      ><button
        @click="
          () => {
            followCrew = false;
            follow = !follow;
            draw();
          }
        "
      >
        Follow depth
      </button>
    </div>
    <div class="world-districts">
      <button
        v-for="[label, x] in [
          ['Camp', 100],
          ['Pit', 459],
          ['Plants', 800],
          ['Waste', 1000],
        ]"
        @click="focusDistrict(Number(x))"
      >
        {{ label }}
      </button>
      <button
        @click="
          followCrew = !followCrew;
          follow = false;
        "
        :aria-pressed="followCrew"
      >
        Follow crew
      </button>
    </div>
    <div class="world-caption">
      {{ state ? "LIVE OPERATION" : "ILLUSTRATIVE PREVIEW" }}
      <span>SCROLL TO ZOOM · DRAG TO EXPLORE</span>
    </div>
  </div>
</template>
