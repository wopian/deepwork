<script setup lang="ts">
import { onMounted, onBeforeUnmount, ref, watch, computed } from "vue";
import { Application, Graphics, Text, Container } from "pixi.js";
// Pixi shader/uniform polyfills preserve the native CSP without eval.
import "pixi.js/unsafe-eval";
import {
  viewState as state,
  materials,
  terrainEpoch,
  format,
  act,
  upgrades,
  snapshotReceivedAt,
  catchup,
  commandsLocked,
} from "./game";
import { motionPosition } from "./visual-timeline";
import { inspectOre } from "./inspect";
import { WasteParticles } from "./waste";
import { TerrainView } from "./terrain-view";
import GameIcon from "./GameIcon.vue";
import { hostRockColour } from "./terrain-pixels";
import {
  campArt,
  headframeArt,
  plantArt,
  workLight,
  workerArt,
} from "./mine-art";
import {
  CELL_PIXEL,
  CELLS_PER_METRE,
  RESOURCE_UNIT,
  MINE_ORIGIN_X,
  SURFACE_Y,
  SHAFT_WORLD_X,
} from "./geometry";
import { routePosition, cargoPosition } from "./routes";
import { preferences, productionAudio } from "./preferences";
import {
  gesture,
  zoomAt,
  resizeViewport,
  fitBounds,
  type ScreenPoint,
} from "./camera";
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
const inspectionPinned = ref(false);
const unknownSignal = ref(false);
const inspectedCell = ref<[number, number] | null>(null);
const accessNotice = computed(() => {
  const game = state.value;
  const cell = inspectedCell.value;
  if (
    !game ||
    !cell ||
    !Number.isFinite(game.access_depth_limit) ||
    cell[1] < game.access_depth_limit * CELLS_PER_METRE
  )
    return null;
  return {
    depth: game.access_depth_limit,
    equipment: (game.access_upgrades ?? [])
      .map((id) => upgrades.find(([key]) => key === id)?.[1] ?? id)
      .join(" + "),
  };
});
let press: ScreenPoint | null = null;
let moved = false;
let offsetX = 0;
let follow = false;
const followCrew = ref(false);
const cameraMenu = ref(false);
let stopWatch: () => void = () => {};

function rect(
  g: Graphics,
  x: number,
  y: number,
  w: number,
  h: number,
  c: number,
) {
  g.rect(
    g === actors ? x : Math.round(x),
    g === actors ? y : Math.round(y),
    w,
    h,
  ).fill(c);
}
function draw() {
  if (!app) return;
  const g = state.value;
  const scale = (app.screen.width / 1100) * zoom;
  if (host.value) {
    host.value.dataset.cameraZoom = String(zoom);
    host.value.dataset.cameraX = String(offsetX);
    host.value.dataset.cameraY = String(offsetY);
    host.value.dataset.cameraFollow = String(follow || followCrew.value);
    host.value.dataset.cameraWidth = String(app.screen.width);
    host.value.dataset.cameraHeight = String(app.screen.height);
  }
  const groundLeft = Math.floor(-offsetX / scale) - 1100;
  const W = Math.ceil(app.screen.width / scale) + 2200;
  const first = Math.max(
    0,
    Math.floor(
      (-offsetY / ((app.screen.width / 1100) * zoom) - SURFACE_Y) / CELL_PIXEL,
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
    JSON.stringify(g?.construction),
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
    const construction = g?.construction;
    if (
      construction &&
      construction.cleared_to.some((p, i) => p !== construction.from[i])
    ) {
      const from = construction.from,
        end = construction.cleared_to;
      const span = Math.max(
        Math.abs(end[0] - from[0]),
        Math.abs(end[1] - from[1]),
        1,
      );
      for (let step = 0; step <= span; step += construction.lift ? 2 : 8) {
        const x =
          MINE_ORIGIN_X +
          (from[0] + ((end[0] - from[0]) * step) / span) * CELL_PIXEL;
        const y =
          SURFACE_Y +
          (from[1] + ((end[1] - from[1]) * step) / span + 1) * CELL_PIXEL;
        if (construction.lift) {
          rect(structures, x - 2, y - 2, 4, 1, 0xa67548);
          rect(structures, x - 2, y - 3, 1, 3, 0x6a4937);
          rect(structures, x + 2, y - 3, 1, 3, 0x6a4937);
        } else {
          const height = Math.max(
            1,
            Math.ceil(8 * construction.support_progress),
          );
          rect(structures, x, y - height, 2, height, 0xa67548);
          rect(structures, x, y - height, 1, height, 0xd8bc7d);
          if (construction.support_progress > 0.5)
            rect(structures, x - 3, y - 8, 7, 1, 0xa67548);
        }
      }
    }
    for (let i = 1; i < workings.passages.length; i++) {
      const node = workings.passages[i]!;
      const parent = workings.passages[node.parent]!;
      if (
        Math.max(node.feet[1], parent.feet[1]) < first ||
        Math.min(node.feet[1], parent.feet[1]) - 16 > last
      )
        continue;
      const x = MINE_ORIGIN_X + node.feet[0] * CELL_PIXEL,
        y = SURFACE_Y + (node.feet[1] + 1) * CELL_PIXEL;
      const px = MINE_ORIGIN_X + parent.feet[0] * CELL_PIXEL,
        py = SURFACE_Y + (parent.feet[1] + 1) * CELL_PIXEL;
      if (scale < 0.25) {
        // Keep the complete connected mine readable when fine textures and supports
        // fall below one screen pixel. This uses excavated passages, never geology.
        structures
          .moveTo(px, py - 4 * CELL_PIXEL)
          .lineTo(x, y - 4 * CELL_PIXEL)
          .stroke({
            width: Math.max(7 * CELL_PIXEL, 1 / scale),
            color: 0x101820,
          });
        continue;
      }
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
        if (node.column) {
          rect(structures, x - 1, y - height, 3, height, color);
          rect(structures, x - 1, y - height, 1, height, 0xc0b38f);
          rect(structures, x - 2, y - 1, 5, 1, 0x40525a);
          if (height > 4)
            structures
              .moveTo(x + 1, y - height + 4)
              .lineTo(x + 4, y - height + 1)
              .stroke({ width: 1, color });
        }
        structures
          .moveTo(px, py - parentHeight)
          .lineTo(x, y - height)
          .stroke({ width: 1, color });
        if (node.column) {
          rect(structures, x - 2, y - height + 1, 4, 1, 0x513b32);
          rect(structures, x - 1, y - height + 2, 1, 1, 0xe8dfc8);
          workLight(structures, x, y - height + 3);
        }
      }
    }
    if (preferences.surveyOverlay) {
      for (const signal of workings.signals) {
        if (
          signal.centre[1] + signal.radius < first ||
          signal.centre[1] - signal.radius > last
        )
          continue;
        const x = MINE_ORIGIN_X + signal.centre[0] * CELL_PIXEL,
          y = SURFACE_Y + signal.centre[1] * CELL_PIXEL;
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
          .moveTo(
            MINE_ORIGIN_X + from[0] * CELL_PIXEL,
            SURFACE_Y + from[1] * CELL_PIXEL,
          )
          .lineTo(
            MINE_ORIGIN_X + section.to[0] * CELL_PIXEL,
            SURFACE_Y + section.to[1] * CELL_PIXEL,
          )
          .stroke({ color: 0xe5a34d, width: 2, alpha: 0.65 });
      }
    }
  }
  terrain.clear();
  rect(
    terrain,
    groundLeft,
    190,
    W,
    (last + 5) * CELL_PIXEL,
    hostRockColour(first),
  );
  for (let band = Math.floor(first / 32) * 32; band <= last; band += 32) {
    const y = SURFACE_Y + band * CELL_PIXEL;
    rect(terrain, groundLeft, y, W, 32 * CELL_PIXEL, hostRockColour(band));
    rect(terrain, groundLeft, y + 31, W, 1, 0x675a4b);
    // Sparse host-rock cracks use global coordinates. No private ore information.
    for (
      let x = Math.floor(groundLeft / 72) * 72;
      scale >= 0.5 && x < groundLeft + W;
      x += 72
    ) {
      const shift = ((Math.imul(x, 17) ^ Math.imul(band, 31)) >>> 0) % 20;
      rect(terrain, x + shift, y + 8, 8, 1, 0x766657);
      rect(terrain, x + shift + 7, y + 9, 3, 1, 0x766657);
    }
  }
  rect(terrain, groundLeft, 188, W, 4, 0x779569);
  rect(terrain, groundLeft, 192, W, 4, 0x526b4e);
  rect(terrain, groundLeft, 196, W, 12, 0xc6a578);
  fineTerrain.update(
    g?.terrain,
    first,
    last,
    (-offsetX / scale - MINE_ORIGIN_X) / CELL_PIXEL - 64,
    ((app.screen.width - offsetX) / scale - MINE_ORIGIN_X) / CELL_PIXEL + 64,
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
    campArt(terrain, x, floors);
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
    headframeArt(terrain, SHAFT_WORLD_X, Math.floor((levels.shaft ?? 0) / 10));
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
    plantArt(
      terrain,
      x,
      height,
      hall.kind,
      modules.length,
      tier,
      !!levels.furnace,
    );
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
  let viewport = { width: app.screen.width, height: app.screen.height };
  app.ticker.add((ticker) => {
    if (!app) return;
    if (
      viewport.width !== app.screen.width ||
      viewport.height !== app.screen.height
    ) {
      const nextViewport = {
        width: app.screen.width,
        height: app.screen.height,
      };
      const next = resizeViewport(
        { zoom, x: offsetX, y: offsetY },
        viewport,
        nextViewport,
      );
      zoom = next.zoom;
      offsetX = next.x;
      offsetY = next.y;
      viewport = nextViewport;
      pointers.clear();
      press = null;
    }
    const frameNow = performance.now();
    if (lastFrameTime > 0) {
      frameIntervals[frameCursor++ % 120] = frameNow - lastFrameTime;
    }
    lastFrameTime = frameNow;
    t += preferences.reducedMotion ? 0 : ticker.deltaTime;
    world.scale.set((app.screen.width / 1100) * zoom);
    if (
      followCrew.value &&
      state.value &&
      (state.value.visual_workers?.length || catchup.value)
    ) {
      const worker =
        state.value.visual_workers.find((p) => p.role === "diggers") ??
        state.value.visual_workers[0];
      const cell = state.value.removed.at(-1);
      const point = worker
        ? motionPosition(
            worker.position,
            worker.legs,
            worker.elapsed_ms,
            catchup.value ? 0 : frameNow - snapshotReceivedAt.value,
            worker.speed,
          ).point
        : cell
          ? [cell.x, cell.y]
          : [256, 0];
      const smoothing = preferences.reducedMotion
        ? 1
        : 1 - Math.exp(-ticker.deltaMS / 180);
      const targetX =
        app.screen.width / 2 -
        (((MINE_ORIGIN_X + point[0] * CELL_PIXEL) * app.screen.width) / 1100) *
          zoom;
      const targetY =
        app.screen.height / 2 -
        (((SURFACE_Y + point[1] * CELL_PIXEL) * app.screen.width) / 1100) *
          zoom;
      offsetX += (targetX - offsetX) * smoothing;
      offsetY += (targetY - offsetY) * smoothing;
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
        inspectionPinned.value = false;
        unknownSignal.value = false;
        inspectedCell.value = null;
        pointers.clear();
        press = null;
        fineTerrain.clear();
        drawnKey = "";
        zoom = innerWidth < 800 ? 6 : 3;
        const startScale = (app.screen.width / 1100) * zoom;
        offsetX = app.screen.width / 2 - SHAFT_WORLD_X * startScale;
        offsetY = app.screen.height * 0.4 - SURFACE_Y * startScale;
        follow = false;
        followCrew.value = true;
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
    if (g?.levels.furnace && !catchup.value) {
      const flicker = preferences.reducedMotion ? 0 : Math.floor(t / 7) % 2;
      rect(actors, 737, 171, 6, 4 + flicker, 0xffd481);
    }
    const crew = g?.crew ?? { diggers: 6, haulers: 3 };
    let shown = 0;
    const workerBudget = low ? 100 : 250;
    const sinceSnapshot = catchup.value
      ? 0
      : frameNow - snapshotReceivedAt.value;
    for (const worker of (g?.visual_workers ?? []).slice(0, workerBudget)) {
      const motion = motionPosition(
        worker.position,
        worker.legs,
        worker.elapsed_ms,
        sinceSnapshot,
        worker.speed,
      );
      let x = MINE_ORIGIN_X + motion.point[0] * CELL_PIXEL - 2;
      let y = SURFACE_Y + (motion.point[1] + 1) * CELL_PIXEL;
      if (
        !worker.legs.length &&
        worker.position[1] === 0 &&
        (worker.role === "operators" || worker.role === "reclaimers")
      ) {
        x = (worker.role === "operators" ? 735 : 980) + (worker.id % 8) * 6;
        y = 185;
      }
      if (worker.activity === "lift") {
        rect(actors, x - 2, y - 9, 1, 10, 0x829b9b);
        rect(actors, x + 6, y - 9, 1, 10, 0x415967);
        rect(actors, x - 2, y, 9, 1, 0xd99756);
      }
      const front =
        !worker.legs.length && worker.role === "diggers"
          ? g?.mining_fronts.find(
              (site) =>
                site.position?.[0] === worker.position[0] &&
                site.position?.[1] === worker.position[1],
            )
          : undefined;
      const facing =
        (worker.legs[0]?.to[0] ?? front?.face?.[0] ?? worker.position[0] + 1) -
        worker.position[0];
      workerArt(
        actors,
        x,
        y,
        worker.role,
        worker.activity,
        t + worker.id * 5,
        facing,
        preferences.reducedMotion,
        low,
      );
      actorTargets.push({ x: x + 2, y: y - 3, panel: "Crew" });
      shown++;
    }
    for (const [role, count] of Object.entries(crew)) {
      for (
        let i = 0;
        i < count &&
        !catchup.value &&
        !g?.visual_workers?.length &&
        shown < workerBudget;
        i++, shown++
      ) {
        let x = 250 + i * 8,
          y = 185;
        if (role === "diggers") {
          const active =
            g?.mining_fronts.filter((front) => front.crew > 0) ?? [];
          const front = active[i % Math.max(1, active.length)];
          const cell = g?.removed[Math.max(0, g.removed.length - 1 - i)];
          const feet =
            front?.position ??
            front?.face ??
            (cell ? [cell.x, cell.y] : undefined);
          if (feet) {
            x = MINE_ORIGIN_X + feet[0]! * CELL_PIXEL + (i % 2) * 3;
            y = SURFACE_Y + (feet[1]! + 1) * CELL_PIXEL;
          }
        } else if (role === "haulers") {
          const cargo = g?.shipments[i % Math.max(1, g.shipments.length)];
          const leg =
            cargo &&
            cargoPosition(cargo.legs ?? [], cargo.duration - cargo.remaining);
          if (leg) {
            x = MINE_ORIGIN_X + leg.point[0] * CELL_PIXEL;
            y = SURFACE_Y + leg.point[1] * CELL_PIXEL;
          } else if (cargo?.path.length) {
            const points: [number, number][] = cargo.path.map(([px, py]) => [
              MINE_ORIGIN_X + px * CELL_PIXEL,
              SURFACE_Y + py * CELL_PIXEL + 7,
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
    const visibleCargo = (catchup.value ? [] : (g?.shipments ?? [])).slice(
      0,
      cargoBudget,
    );
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
            MINE_ORIGIN_X + x * CELL_PIXEL,
            SURFACE_Y + y * CELL_PIXEL,
          ])
        : [
            [
              SHAFT_WORLD_X,
              SURFACE_Y + cargo.depth * CELLS_PER_METRE * CELL_PIXEL,
            ],
          ];
      points.push([points[points.length - 1]![0], 185], [735, 185]);
      const leg = cargoPosition(
        cargo.legs ?? [],
        cargo.duration - cargo.remaining,
      );
      const smooth = cargo.id
        ? motionPosition(
            cargo.legs[0]?.from ?? [0, 0],
            cargo.legs,
            cargo.elapsed_ms,
            sinceSnapshot,
            cargo.speed,
          )
        : null;
      const displayedLeg = smooth ?? leg;
      const [x, y] = displayedLeg
        ? [
            MINE_ORIGIN_X + displayedLeg.point[0] * CELL_PIXEL,
            SURFACE_Y + displayedLeg.point[1] * CELL_PIXEL,
          ]
        : routePosition(points, progress);
      const mode = displayedLeg?.mode ?? cargo.mode;
      actorTargets.push({ x: x + 4, y: y + 2, panel: "Logistics" });
      const oreColor = parseInt(materials[cargo.material].color.slice(1), 16);
      if (mode === "train") {
        rect(actors, x - 8, y + 2, 16, 4, 0x8c9ba5);
        rect(actors, x - 5, y, 9, 3, oreColor);
        rect(actors, x - 6, y + 6, 3, 1, 0xe8dfc8);
        rect(actors, x + 3, y + 6, 3, 1, 0xe8dfc8);
      } else if (mode === "minecart") {
        rect(actors, x - 5, y + 2, 10, 3, 0x8c9ba5);
        rect(actors, x - 3, y, 6, 3, oreColor);
        rect(actors, x - 3, y + 5, 2, 1, 0xe8dfc8);
        rect(actors, x + 1, y + 5, 2, 1, 0xe8dfc8);
      } else if (mode === "lift") {
        rect(actors, x - 3, y - 4, 1, 7, 0x8c9ba5);
        rect(actors, x + 2, y - 4, 1, 7, 0x8c9ba5);
        rect(actors, x - 3, y + 2, 6, 1, 0x8c9ba5);
        rect(actors, x - 2, y - 1, 4, 3, oreColor);
      } else if (mode === "wheelbarrow") {
        rect(actors, x - 3, y + 1, 6, 2, 0x8c9ba5);
        rect(actors, x - 1, y - 1, 3, 2, oreColor);
        rect(actors, x + 1, y + 3, 2, 1, 0xe8dfc8);
      } else if (mode === "conveyor") {
        rect(actors, x - 2, y, 4, 3, oreColor);
      } else {
        rect(actors, x - 1, y, 2, 2, oreColor);
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
  followCrew.value = false;
  zoom = innerWidth < 800 ? 4 : 2;
  const scale = (app.screen.width / 1100) * zoom;
  offsetY = Math.max(150, app.screen.height * 0.45) - SURFACE_Y * scale;
  offsetX = app.screen.width / 2 - x * scale;
  draw();
}
function surfaceOverview() {
  if (!app) return;
  zoom = innerWidth < 800 ? 2 : 1;
  const scale = (app.screen.width / 1100) * zoom;
  offsetX = app.screen.width / 2 - 550 * scale;
  offsetY = 150 - SURFACE_Y * scale;
  follow = false;
  followCrew.value = false;
  draw();
}
function focusCrew() {
  zoom = Math.max(zoom, innerWidth < 800 ? 6 : 3);
  followCrew.value = true;
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
  followCrew.value = false;
  draw();
}
function pointerDown(e: PointerEvent) {
  pointers.set(e.pointerId, localPoint(e));
  if (pointers.size === 1) {
    press = localPoint(e);
    moved = false;
  } else moved = true;
  follow = false;
  followCrew.value = false;
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
  if (!select && inspectionPinned.value) return;
  const scale = (app.screen.width / 1100) * zoom;
  inspectedCell.value = [
    Math.floor(((p.x - offsetX) / scale - MINE_ORIGIN_X) / CELL_PIXEL),
    Math.floor(((p.y - offsetY) / scale - SURFACE_Y) / CELL_PIXEL),
  ];
  inspected.value = inspectOre(state.value.terrain, ...inspectedCell.value);
  if (select) inspectionPinned.value = inspected.value !== null;
  unknownSignal.value =
    inspected.value === null &&
    state.value.workings.signals.some(
      (s) =>
        Math.hypot(
          s.centre[0] - inspectedCell.value![0],
          s.centre[1] - inspectedCell.value![1],
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
  if (select && wy >= 100 && wy < SURFACE_Y) {
    emit(
      "inspect",
      wx < MINE_ORIGIN_X ? "Crew" : wx < 680 ? "Logistics" : "Processing",
    );
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
    focusDistrict(SHAFT_WORLD_X);
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
  const next = fitBounds(
    {
      left: MINE_ORIGIN_X + minX * CELL_PIXEL,
      right: MINE_ORIGIN_X + maxX * CELL_PIXEL,
      top: SURFACE_Y,
      bottom: SURFACE_Y + maxY * CELL_PIXEL,
    },
    app.screen,
  );
  zoom = next.zoom;
  offsetX = next.x;
  offsetY = next.y;
  follow = false;
  followCrew.value = false;
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
      <button @click="changeZoom(1.4)" aria-label="Zoom in">
        <GameIcon name="plus" />
      </button>
      <button @click="changeZoom(1 / 1.4)" aria-label="Zoom out">
        <GameIcon name="minus" />
      </button>
      <button
        @click="cameraMenu = !cameraMenu"
        aria-label="Camera controls"
        :aria-expanded="cameraMenu"
      >
        <GameIcon name="camera" />
      </button>
    </div>
    <section
      v-if="cameraMenu"
      class="camera-menu"
      aria-label="Camera controls"
      @keydown.esc.stop="cameraMenu = false"
    >
      <h2>Explore the worksite</h2>
      <button
        @click="
          surfaceOverview();
          cameraMenu = false;
        "
      >
        <GameIcon name="surface" />Surface
      </button>
      <button
        @click="
          focusCrew();
          cameraMenu = false;
        "
      >
        <GameIcon name="Crew" />Active crew
      </button>
      <button
        @click="
          fitWorkings();
          cameraMenu = false;
        "
      >
        <GameIcon name="camera" />Fit workings
      </button>
      <button
        @click="
          followCrew = !followCrew;
          follow = false;
        "
        :aria-pressed="followCrew"
      >
        <GameIcon name="follow" />Follow crew
      </button>
      <button
        @click="
          preferences.surveyOverlay = !preferences.surveyOverlay;
          drawnKey = '';
          draw();
        "
        :aria-pressed="preferences.surveyOverlay"
        title="One notch: possible. Two notches: promising. Only sampled ore appears in rock."
      >
        <GameIcon name="survey" />Survey / work plan
      </button>
      <div class="camera-districts">
        <button
          v-for="[label, x] in [
            ['Camp', 100],
            ['Shaft', SHAFT_WORLD_X],
            ['Plants', 800],
            ['Waste', 1000],
          ]"
          @click="
            focusDistrict(Number(x));
            cameraMenu = false;
          "
        >
          {{ label }}
        </button>
      </div>
    </section>
    <div class="world-caption">
      {{ state ? "Crew at work" : "Mine preview" }}
    </div>
    <div v-if="unknownSignal" class="ore-inspector" role="status">
      <strong>Unknown mineral signal</strong
      ><span>Survey crew needs more samples to identify this deposit.</span>
    </div>
    <div v-if="inspected !== null" class="ore-inspector" role="status">
      <strong>{{ materials[inspected]?.name }}</strong>
      <span v-if="!inspectionPinned"
        >Click an ore face to keep this inspector open.</span
      >
      <span
        >Refines into
        {{ materials[inspected]?.product.replaceAll("_", " ") }}</span
      >
      <span v-if="accessNotice">
        Access ends at {{ format(accessNotice.depth) }} m.
        <template v-if="accessNotice.equipment"
          >Needed: {{ accessNotice.equipment }}.</template
        >
        <template v-else>Current lift is at its maximum reach.</template>
        Crews can still work accessible parts of this vein.
      </span>
      <button v-if="accessNotice" @click="emit('inspect', 'Equipment')">
        View access equipment
      </button>
      <button
        :disabled="commandsLocked"
        @click="inspectedCell && act('target_vein', inspectedCell.join(','))"
      >
        Prioritise whole vein
      </button>
      <button
        :disabled="commandsLocked"
        v-if="state?.workings?.target"
        @click="act('clear_vein')"
      >
        Clear vein order
      </button>
      <button
        :disabled="commandsLocked"
        @click="act('priority', '', inspected)"
      >
        {{
          state?.priorities.includes(inspected)
            ? "Remove material priority"
            : "Prioritise this material"
        }}
      </button>
      <button
        @click="
          inspected = null;
          inspectionPinned = false;
        "
        aria-label="Close mineral inspector"
      >
        ×
      </button>
    </div>
  </div>
</template>
