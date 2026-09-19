<script setup lang="ts">
import { onMounted, onBeforeUnmount, ref, watch } from "vue";
import { Application, Graphics, Text, Container } from "pixi.js";
// Pixi shader/uniform polyfills preserve the native CSP without eval.
import "pixi.js/unsafe-eval";
import { state, materials, terrainEpoch, format, act } from "./game";
import { inspectOre } from "./inspect";
import { WasteParticles } from "./waste";
import { TerrainView } from "./terrain-view";
import { CELL_PIXEL, CELLS_PER_METRE, RESOURCE_UNIT } from "./geometry";
import { routePosition, cargoPosition } from "./routes";
import { preferences, productionAudio } from "./preferences";
import { gesture, zoomAt, type ScreenPoint } from "./camera";
const emit = defineEmits<{ inspect: [panel: string] }>();
const host = ref<HTMLDivElement>();
let app: Application | undefined;
let world: Container;
let terrain: Graphics;
const fineTerrain = new TerrainView();
let actors: Graphics;
const actorTargets: { x: number; y: number; panel: string }[] = [];
let structures: Graphics;
let wasteLabel: Text;
let t = 0;
let telemetryTime = 0;
let telemetryFrames = 0;
let lastFrameTime = 0;
const frameIntervals: number[] = [];
let frameCursor = 0;
const wasteParticles = new WasteParticles(600);
let drawnKey = "";
let lastWaste = 0;
let lastSite = 0;
let lastCampaign = "";
let zoom = 1;
let offsetY = 0;
const pointers = new Map<number, ScreenPoint>();
const inspected = ref<number | null>(null);
const unknownSignal = ref(false);
let inspectedCell: [number, number] | null = null;
let press: ScreenPoint | null = null;
let moved = false;
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
  const scale = (app.screen.width / 1100) * zoom;
  if (host.value) {
    host.value.dataset.cameraZoom = String(zoom);
    host.value.dataset.cameraX = String(offsetX);
    host.value.dataset.cameraY = String(offsetY);
    host.value.dataset.cameraFollow = String(follow || followCrew);
  }
  const groundLeft = Math.floor(-offsetX / scale) - 1100;
  const W = Math.ceil(app.screen.width / scale) + 2200;
  const first = Math.max(
    0,
    Math.floor(
      (-offsetY / ((app.screen.width / 1100) * zoom) - 208) / CELL_PIXEL,
    ) - 30,
  );
  const last =
    first +
    Math.ceil(
      app.screen.height / ((app.screen.width / 1100) * zoom) / CELL_PIXEL,
    ) +
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
    g?.workings?.revision,
    preferences.surveyOverlay,
    g?.housing,
    first,
    last,
    groundLeft,
    W,
    JSON.stringify(g?.waste_profile),
    Math.floor((g?.lifetime_waste ?? 0) / RESOURCE_UNIT),
    JSON.stringify(g?.levels),
  ].join(":");
  if (key === drawnKey) return;
  drawnKey = key;
  structures.clear();
  const workings = g?.workings;
  if (workings) {
    for (let i = 1; i < workings.passages.length; i++) {
      const node = workings.passages[i]!;
      const parent = workings.passages[node.parent]!;
      if (
        Math.max(node.feet[1], parent.feet[1]) < first ||
        Math.min(node.feet[1], parent.feet[1]) - 16 > last
      )
        continue;
      const x = 235 + node.feet[0] * CELL_PIXEL,
        y = 208 + (node.feet[1] + 1) * CELL_PIXEL;
      const px = 235 + parent.feet[0] * CELL_PIXEL,
        py = 208 + (parent.feet[1] + 1) * CELL_PIXEL;
      if (node.lift) {
        rect(
          structures,
          x - 2 * CELL_PIXEL,
          Math.min(y, py) - 7 * CELL_PIXEL,
          1,
          Math.abs(y - py) + 7 * CELL_PIXEL,
          0x8c9ba5,
        );
        rect(
          structures,
          x + 2 * CELL_PIXEL,
          Math.min(y, py) - 7 * CELL_PIXEL,
          1,
          Math.abs(y - py) + 7 * CELL_PIXEL,
          0x8c9ba5,
        );
        rect(
          structures,
          x - 2 * CELL_PIXEL,
          y - 1,
          4 * CELL_PIXEL,
          1,
          0xe5a34d,
        );
      } else if (node.supported) {
        const color = (g?.levels.supports ?? 0) > 0 ? 0x6e7778 : 0xa67548;
        const height =
          (node.feet[1] - (workings.chambers?.[i] ?? node.feet[1] - 7) + 1) *
          CELL_PIXEL;
        const parentHeight =
          (parent.feet[1] -
            (workings.chambers?.[node.parent] ?? parent.feet[1] - 7) +
            1) *
          CELL_PIXEL;
        if (node.column) rect(structures, x - 1, y - height, 1, height, color);
        structures
          .moveTo(px, py - parentHeight)
          .lineTo(x, y - height)
          .stroke({ width: 1, color });
      }
    }
    if (preferences.surveyOverlay) {
      for (const signal of workings.signals) {
        if (
          signal.centre[1] + signal.radius < first ||
          signal.centre[1] - signal.radius > last
        )
          continue;
        const x = 235 + signal.centre[0] * CELL_PIXEL,
          y = 208 + signal.centre[1] * CELL_PIXEL;
        structures
          .circle(x, y, signal.radius * CELL_PIXEL)
          .fill({ color: 0xe5a34d, alpha: 0.1 })
          .stroke({ color: 0xe5a34d, width: 1, alpha: 0.6 });
        // One/two notches convey confidence independently of colour.
        for (let n = 0; n < signal.confidence; n++)
          rect(structures, x - 2 + n * 4, y - 2, 2, 4, 0xe8dfc8);
      }
      const section = workings.section;
      if (section) {
        const from = workings.passages[section.from]!.feet;
        structures
          .moveTo(235 + from[0] * CELL_PIXEL, 208 + from[1] * CELL_PIXEL)
          .lineTo(
            235 + section.to[0] * CELL_PIXEL,
            208 + section.to[1] * CELL_PIXEL,
          )
          .stroke({ color: 0xe5a34d, width: 2, alpha: 0.65 });
      }
    }
  }
  terrain.clear();
  rect(terrain, groundLeft, 190, W, (last + 5) * CELL_PIXEL, 0x806044);
  rect(terrain, groundLeft, 188, W, 8, 0x6b8f47);
  rect(terrain, groundLeft, 196, W, 12, 0xd8bc7d);
  fineTerrain.update(
    g?.terrain,
    first,
    last,
    (-offsetX / scale - 235) / CELL_PIXEL - 64,
    ((app.screen.width - offsetX) / scale - 235) / CELL_PIXEL + 64,
    g?.selected_vein?.masks,
  );
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
  if (g || levels.shaft) {
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
  wasteLabel.text = `LIFETIME ${format((g?.lifetime_waste ?? 0) / RESOURCE_UNIT)} units`;
  const pile = g?.waste_profile;
  if (pile)
    for (let i = 0; i < pile.heights.length; i++) {
      const height = pile.heights[i]! / 1024;
      if (height <= 0) continue;
      const x = 1000 + pile.origin + i * pile.pitch;
      rect(terrain, x, 190 - height, pile.pitch, Math.ceil(height), 0x8c9ba5);
      if (height > 5) rect(terrain, x, 193 - height, pile.pitch, 1, 0x806044);
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
  structures = new Graphics();
  world.addChild(terrain, fineTerrain.layer, structures, actors);
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
    const frameNow = performance.now();
    if (lastFrameTime > 0) {
      frameIntervals[frameCursor++ % 120] = frameNow - lastFrameTime;
    }
    lastFrameTime = frameNow;
    t += preferences.reducedMotion ? 0 : ticker.deltaTime;
    world.scale.set((app.screen.width / 1100) * zoom);
    if (followCrew && state.value?.removed.length) {
      const cell = state.value.removed[state.value.removed.length - 1]!;
      offsetX =
        app.screen.width / 2 -
        (((235 + cell.x * CELL_PIXEL) * app.screen.width) / 1100) * zoom;
      offsetY =
        app.screen.height / 2 -
        (((208 + cell.y * CELL_PIXEL) * app.screen.width) / 1100) * zoom;
    } else if (follow) {
      offsetY =
        80 -
        ((Object.values(state.value?.heights ?? {}).reduce(
          (max, h) => Math.max(max, h),
          0,
        ) *
          CELL_PIXEL *
          app.screen.width) /
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
    actorTargets.length = 0;
    const g = state.value;
    productionAudio(g?.stages.reduce((sum, s) => sum + s.rate, 0) ?? 0);
    if (g) {
      const worldIdentity = `${g.campaign_id}:${g.site}`;
      if (lastCampaign !== worldIdentity) {
        lastCampaign = worldIdentity;
        inspected.value = null;
        unknownSignal.value = false;
        inspectedCell = null;
        pointers.clear();
        press = null;
        fineTerrain.clear();
        drawnKey = "";
        zoom = innerWidth < 800 ? 6 : 3;
        const startScale = (app.screen.width / 1100) * zoom;
        offsetX = app.screen.width / 2 - 459 * startScale;
        offsetY = app.screen.height * 0.4 - 208 * startScale;
        follow = false;
        followCrew = true;
        wasteParticles.items.length = 0;
        lastWaste = g.lifetime_waste;
      }
      if (lastSite !== g.site) {
        lastSite = g.site;
        lastWaste = g.lifetime_waste;
        wasteParticles.items.length = 0;
      }
      if (!preferences.reducedMotion)
        wasteParticles.emit(
          Math.ceil(
            Math.max(0, g.lifetime_waste - lastWaste) / (RESOURCE_UNIT / 20),
          ),
          1000 +
            g.waste_profile.origin +
            g.waste_profile.discharge * g.waste_profile.pitch,
          170 -
            (g.waste_profile.heights[g.waste_profile.discharge] ?? 0) / 1024,
        );
      lastWaste = g.lifetime_waste;
    }
    if (preferences.reducedMotion) wasteParticles.items.length = 0;
    wasteParticles.step(ticker.deltaTime, (x) => {
      const pile = state.value?.waste_profile;
      if (!pile) return 188;
      const index = Math.floor((x - 1000 - pile.origin) / pile.pitch);
      return 188 - (pile.heights[index] ?? 0) / 1024;
    });
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
            const feet = g?.work_route?.[0] ?? [cell.x, cell.y];
            x = 235 + feet[0]! * CELL_PIXEL;
            y = 208 + (feet[1]! + 1) * CELL_PIXEL;
          }
        } else if (role === "haulers") {
          const cargo = g?.shipments[i % Math.max(1, g.shipments.length)];
          const leg =
            cargo &&
            cargoPosition(cargo.legs ?? [], cargo.duration - cargo.remaining);
          if (leg) {
            x = 235 + leg.point[0] * CELL_PIXEL;
            y = 208 + leg.point[1] * CELL_PIXEL;
          } else if (cargo?.path.length) {
            const points: [number, number][] = cargo.path.map(([px, py]) => [
              235 + px * CELL_PIXEL,
              208 + py * CELL_PIXEL + 7,
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
        actorTargets.push({ x: x + 2, y: y - 3, panel: "Crew" });
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
      const ordered = [...frameIntervals].sort((a, b) => a - b);
      host.value.dataset.frameP50 = (
        ordered[Math.floor(ordered.length * 0.5)] ?? 0
      ).toFixed(2);
      host.value.dataset.frameP95 = (
        ordered[
          Math.min(ordered.length - 1, Math.floor(ordered.length * 0.95))
        ] ?? 0
      ).toFixed(2);
      telemetryFrames = 0;
      telemetryTime = now;
      host.value.dataset.workers = String(shown);
      host.value.dataset.particles = String(wasteParticles.items.length);
    }
    const cargoBudget = Math.max(
      0,
      (low ? 600 : 2000) - wasteParticles.items.length,
    );
    const visibleCargo = (g?.shipments ?? []).slice(0, cargoBudget);
    if (host.value) {
      host.value.dataset.cargo = String(visibleCargo.length);
      host.value.dataset.moving = String(
        visibleCargo.length + wasteParticles.items.length,
      );
      host.value.dataset.residentChunks = String(fineTerrain.residentChunks);
    }
    for (const cargo of visibleCargo) {
      const progress = 1 - cargo.remaining / cargo.duration;
      const points: [number, number][] = cargo.path?.length
        ? cargo.path.map(([x, y]) => [
            235 + x * CELL_PIXEL,
            208 + y * CELL_PIXEL,
          ])
        : [[459, 208 + cargo.depth * CELLS_PER_METRE * CELL_PIXEL]];
      points.push([points[points.length - 1]![0], 185], [735, 185]);
      const leg = cargoPosition(
        cargo.legs ?? [],
        cargo.duration - cargo.remaining,
      );
      const [x, y] = leg
        ? [235 + leg.point[0] * CELL_PIXEL, 208 + leg.point[1] * CELL_PIXEL]
        : routePosition(points, progress);
      const mode = leg?.mode ?? cargo.mode;
      actorTargets.push({ x: x + 4, y: y + 2, panel: "Logistics" });
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
  fineTerrain.clear();
  productionAudio(0);
  stopWatch();
  app?.destroy(true, { children: true });
});
function focusDistrict(x: number) {
  if (!app) return;
  follow = false;
  followCrew = false;
  zoom = innerWidth < 800 ? 4 : 2;
  const scale = (app.screen.width / 1100) * zoom;
  offsetY = Math.max(150, app.screen.height * 0.45) - 208 * scale;
  offsetX = app.screen.width / 2 - x * scale;
  draw();
}
function surfaceOverview() {
  if (!app) return;
  zoom = innerWidth < 800 ? 2 : 1;
  const scale = (app.screen.width / 1100) * zoom;
  offsetX = app.screen.width / 2 - 550 * scale;
  offsetY = 150 - 208 * scale;
  follow = false;
  followCrew = false;
  draw();
}
function focusCrew() {
  zoom = Math.max(zoom, innerWidth < 800 ? 6 : 3);
  followCrew = true;
  follow = false;
  draw();
}
function wheel(e: WheelEvent) {
  changeZoom(Math.exp(-e.deltaY * 0.002), localPoint(e));
}
function localPoint(e: { clientX: number; clientY: number }): ScreenPoint {
  const bounds = host.value!.getBoundingClientRect();
  return {
    x:
      ((e.clientX - bounds.left) * (app?.screen.width ?? bounds.width)) /
      bounds.width,
    y:
      ((e.clientY - bounds.top) * (app?.screen.height ?? bounds.height)) /
      bounds.height,
  };
}
function changeZoom(
  factor: number,
  from = { x: (app?.screen.width ?? 0) / 2, y: (app?.screen.height ?? 0) / 2 },
  to = from,
) {
  const next = zoomAt({ zoom, x: offsetX, y: offsetY }, factor, from, to);
  zoom = next.zoom;
  offsetX = next.x;
  offsetY = next.y;
  follow = false;
  followCrew = false;
  draw();
}
function pointerDown(e: PointerEvent) {
  pointers.set(e.pointerId, localPoint(e));
  if (pointers.size === 1) {
    press = localPoint(e);
    moved = false;
  } else moved = true;
  follow = false;
  followCrew = false;
  (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
}
function pointerMove(e: PointerEvent) {
  if (!pointers.has(e.pointerId)) {
    if (e.pointerType === "mouse") inspectAt(localPoint(e));
    return;
  }
  if (
    press &&
    Math.hypot(localPoint(e).x - press.x, localPoint(e).y - press.y) > 6
  )
    moved = true;
  const before = gesture([...pointers.values()]);
  pointers.set(e.pointerId, localPoint(e));
  const after = gesture([...pointers.values()]);
  changeZoom(
    before.distance > 0 && after.distance > 0
      ? after.distance / before.distance
      : 1,
    before.centre,
    after.centre,
  );
}
function inspectAt(p: ScreenPoint, select = false) {
  if (!app || !state.value) return;
  const scale = (app.screen.width / 1100) * zoom;
  inspectedCell = [
    Math.floor(((p.x - offsetX) / scale - 235) / CELL_PIXEL),
    Math.floor(((p.y - offsetY) / scale - 208) / CELL_PIXEL),
  ];
  inspected.value = inspectOre(state.value.terrain, ...inspectedCell);
  unknownSignal.value =
    inspected.value === null &&
    state.value.workings.signals.some(
      (s) =>
        Math.hypot(
          s.centre[0] - inspectedCell![0],
          s.centre[1] - inspectedCell![1],
        ) <= s.radius,
    );
  const wx = (p.x - offsetX) / scale,
    wy = (p.y - offsetY) / scale;
  if (select && inspected.value === null) {
    const radius = 22 / scale;
    const target = actorTargets.reduce<{
      x: number;
      y: number;
      panel: string;
    } | null>((best, candidate) => {
      const distance = Math.hypot(candidate.x - wx, candidate.y - wy);
      return distance <= radius &&
        (!best || distance < Math.hypot(best.x - wx, best.y - wy))
        ? candidate
        : best;
    }, null);
    if (target) {
      emit("inspect", target.panel);
      return;
    }
  }
  if (select && wy >= 100 && wy < 208) {
    emit("inspect", wx < 235 ? "Crew" : wx < 680 ? "Logistics" : "Processing");
  }
}
function pointerEnd(e: PointerEvent) {
  if (e.type === "pointerup" && pointers.has(e.pointerId) && !moved)
    inspectAt(localPoint(e), true);
  pointers.delete(e.pointerId);
}
function fitWorkings() {
  if (!app) return;
  const points = state.value?.workings.passages.map((p) => p.feet) ?? [];
  if (!points.length) {
    focusDistrict(459);
    return;
  }
  let minX = Infinity,
    maxX = -Infinity,
    maxY = 0;
  for (const [x, y] of points) {
    minX = Math.min(minX, x);
    maxX = Math.max(maxX, x);
    maxY = Math.max(maxY, y);
  }
  const scale = Math.min(
    (app.screen.width - 64) / Math.max(100, (maxX - minX) * CELL_PIXEL),
    (app.screen.height - 224) / Math.max(100, maxY * CELL_PIXEL),
  );
  zoom = Math.max(0.15, Math.min(24, (scale * 1100) / app.screen.width));
  const actual = (app.screen.width / 1100) * zoom;
  offsetX =
    app.screen.width / 2 - (235 + ((minX + maxX) / 2) * CELL_PIXEL) * actual;
  offsetY = 128 - 208 * actual;
  follow = false;
  followCrew = false;
  draw();
}
</script>
<template>
  <div class="world-wrap">
    <div
      ref="host"
      class="world"
      @wheel.prevent="wheel"
      @pointerdown="pointerDown"
      @pointermove="pointerMove"
      @pointerup="pointerEnd"
      @pointercancel="pointerEnd"
      @lostpointercapture="pointerEnd"
    />
    <div class="world-tools">
      <button @click="changeZoom(1.4)" aria-label="Zoom in">＋</button
      ><button @click="changeZoom(1 / 1.4)" aria-label="Zoom out">−</button
      ><button @click="surfaceOverview">Surface ↑</button>
      <button @click="focusCrew">Active crew</button>
      <button @click="fitWorkings">Fit workings</button>
    </div>
    <div class="world-districts">
      <button
        v-for="[label, x] in [
          ['Camp', 100],
          ['Shaft', 459],
          ['Plants', 800],
          ['Waste', 1000],
        ]"
        @click="focusDistrict(Number(x))"
      >
        {{ label }}
      </button>
      <button
        @click="
          preferences.surveyOverlay = !preferences.surveyOverlay;
          drawnKey = '';
          draw();
        "
        :aria-pressed="preferences.surveyOverlay"
        title="Approximate signals: one notch = possible; two = promising. Sampled ore appears in terrain."
      >
        Survey / work plan
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
      <span>{{
        state?.workings?.status || "SCROLL TO ZOOM · DRAG TO EXPLORE"
      }}</span>
    </div>
    <div v-if="unknownSignal" class="ore-inspector" role="status">
      <strong>Unknown mineral signal</strong
      ><span>Survey crew needs more samples to identify this deposit.</span>
    </div>
    <div v-if="inspected !== null" class="ore-inspector" role="status">
      <strong>{{ materials[inspected]?.name }}</strong>
      <span
        >Refines into
        {{ materials[inspected]?.product.replaceAll("_", " ") }}</span
      >
      <button
        @click="inspectedCell && act('target_vein', inspectedCell.join(','))"
      >
        Prioritise whole vein
      </button>
      <button v-if="state?.workings?.target" @click="act('clear_vein')">
        Clear vein order
      </button>
      <button @click="act('priority', '', inspected)">
        {{
          state?.priorities.includes(inspected)
            ? "Remove material priority"
            : "Prioritise this material"
        }}
      </button>
      <button @click="inspected = null" aria-label="Close mineral inspector">
        ×
      </button>
    </div>
  </div>
</template>
