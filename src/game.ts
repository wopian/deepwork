import { shallowRef, computed, watch } from "vue";
import { Channel, invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import catalogue from "../content/materials.json";
import { displayNumber } from "./numbers";
import { preferences, purchaseSound } from "./preferences";
import { mergeWorldUpdate, latestWorldSnapshot } from "./visual-timeline";
import type { CargoLeg } from "./routes";
export const materials = catalogue;
export interface Game {
  campaign_id: string;
  save_status: {
    last_success: number;
    bytes: number;
    format: number;
    error: string;
  };
  requires_reset: boolean;
  enabled_recipes: string[];
  paused_recipes: string[];
  megaproject: boolean;
  site_discoveries: number;
  version: number;
  seed: string;
  site: number;
  profile: number;
  challenge: string;
  trace_feed: Record<string, number>;
  ticks: number;
  credits: string;
  workers: number;
  visual_workers: {
    id: number;
    role: string;
    job: string;
    position: [number, number];
    activity: string;
    route: number;
    elapsed_ms: number;
    speed: number;
    legs: CargoLeg[];
  }[];
  crew_state: {
    assigned: number;
    travelling: number;
    working: number;
    blocked: number;
  };
  crew_roles: Record<
    string,
    { assigned: number; travelling: number; working: number; blocked: number }
  >;
  construction: null | {
    from: [number, number];
    to: [number, number];
    cleared_to: [number, number];
    lift: boolean;
    support_progress: number;
  };
  housing: number;
  levels: Record<string, number>;
  milestones: string[];
  build_queue: string[];
  research: number;
  ranks: Record<string, number>;
  policy: string;
  specialisation: string | null;
  priorities: number[];
  reserve: Record<string, number>;
  pinned: string | null;
  work_route: [number, number][];
  raw_stock: Record<string, number>;
  concentrate: Record<string, number>;
  raw_stock_capacity: number;
  research_invested: number;
  shipments: {
    id: string;
    route: number;
    elapsed_ms: number;
    speed: number;
    material: number;
    amount: number;
    remaining: number;
    duration: number;
    depth: number;
    mode: string;
    path: [number, number][];
    legs: {
      from: [number, number];
      to: [number, number];
      mode: string;
      milliseconds: number;
    }[];
  }[];
  transport: {
    express: number;
    stations: {
      id: string;
      name: string;
      level: number;
      capacity: number;
      cargo: Record<string, number>;
      preferred: boolean;
      incoming: number;
      outgoing: number;
      quote: string;
    }[];
    segments: {
      name: string;
      capacity: number;
      blocked: boolean;
      blocker: string;
      utilisation: number;
      demand: number;
      batches: { amount: number }[];
      duration_ms: number;
      rate: number;
    }[];
  };
  crew: Record<string, number>;
  crew_priority: string;
  cargo_policy: string;
  mining_fronts: {
    id: string;
    deposit: string;
    material: number;
    face: [number, number];
    position: [number, number];
    crew: number;
    haulers: number;
    progress: number;
    cut_work: number;
    stockpile: number;
    stockpiles: Record<number, number>;
    capacity: number;
    route: [number, number][];
    selected: boolean;
    status: string;
    blocker: string;
  }[];
  workings_offset: number;
  workings: {
    target: [number, number] | null;
    revision: number;
    status: string;
    active: number;
    chambers: Record<number, number>;
    passages: {
      feet: [number, number];
      parent: number;
      lift: boolean;
      supported: boolean;
      column: boolean;
    }[];
    signals: { centre: [number, number]; radius: number; confidence: number }[];
    section: {
      from: number;
      to: [number, number];
      lift: boolean;
      support_work: number;
    } | null;
  };
  selected_vein: {
    anchor: [number, number];
    known_cells: number;
    masks: Record<string, number[]>;
  } | null;
  pinned_inputs: Record<string, number>;
  access_depth_limit: number;
  access_upgrades: string[];
  processing: {
    id: number;
    output: string;
    intake: number;
    stored: number;
    transit: number;
    buffered: number;
    queued: number;
    product: number;
    reserve_target: number;
    reserved: number;
    input_rate: number;
    output_rate: number;
    recovery_percent: number;
    blocker: string;
    destination: string;
  }[];
  heights: Record<string, number>;
  terrain: {
    chunks: Record<string, number[]>;
    revealed: Record<string, number[]>;
    visible: Record<string, number[]>;
    revision: number;
  };
  removed: { x: number; y: number; material: number }[];
  ore: Record<string, number>;
  hauled: Record<string, number>;
  products: Record<string, number>;
  tailings: Record<string, number>;
  slag: number;
  depleted: number;
  lifetime_waste: number;
  waste_profile: {
    heights: number[];
    pitch: number;
    origin: number;
    discharge: number;
  };
  excavated: number;
  discoveries: number[];
  collection: string[];
  quotes: Record<string, string>;
  purchase_blockers: Record<string, string>;
  upgrade_previews: Record<
    string,
    {
      machine_percent: number;
      line_percent: number;
      lift_depth_after?: number | null;
      access_depth_after?: number | null;
    }
  >;
  retirement_award: number;
  site_objectives: string[];
  contracts: { product: string; amount: number; complete: boolean }[];
  records: {
    site: number;
    section: number[];
    depth: number;
    research: number;
    excavated: number;
  }[];
  last_sequence: number;
  stages: {
    name: string;
    rate: number;
    buffer: number;
    capacity: number;
    blocker: string;
  }[];
  offline: null | {
    id: string;
    discoveries: number[];
    blockers: string[];
    capped: number;
    elapsed: number;
    effective: number;
    credits: string;
    excavated: number;
  };
  steel_made: boolean;
}
export const state = shallowRef<Game | null>(null);
export const visualState = shallowRef<Game | null>(null);
export const viewState = computed(() => visualState.value ?? state.value);
export const snapshotReceivedAt = shallowRef(0);
export const catchup = shallowRef<{
  session: string;
  done: number;
  total: number;
  skipped: boolean;
  saving: boolean;
  baselineExcavated: number;
  excavated: number;
} | null>(null);
const importing = shallowRef(false);
export const commandsLocked = computed(
  () => importing.value || reconciling.value || catchup.value !== null,
);
type VisualEvent =
  | { kind: "start"; session: string; state: Game; total: number }
  | {
      kind: "frame";
      session: string;
      update: { state: Game; reset: boolean };
      done: number;
      total: number;
    }
  | { kind: "saved"; session: string; state: Game }
  | { kind: "failed"; session: string; message: string; state: Game };
let frames: Extract<VisualEvent, { kind: "frame" }>[] = [];
let frameBytes = 0;
let savedResult: Game | null = null;
let liveResult: Game | null = null;
let playbackScheduled = false;
function finishPlayback() {
  const session = catchup.value?.session;
  if (!session || !savedResult) return;
  state.value = liveResult ?? savedResult;
  liveResult = null;
  visualState.value = null;
  snapshotReceivedAt.value = performance.now();
  catchup.value = null;
  savedResult = null;
  frames = [];
  frameBytes = 0;
  reconciling.value = true;
  terrainEpoch.value++;
  void invoke("acknowledge_replay", { session })
    .then(() => {
      reconciling.value = false;
    })
    .catch((e) => {
      error.value = String(e);
    });
}
function consumeFrame() {
  playbackScheduled = false;
  if (!catchup.value) return;
  const frame = frames.shift();
  if (frame && !catchup.value.skipped) {
    frameBytes -= JSON.stringify(frame).length * 2;
    visualState.value = mergeWorldUpdate(
      visualState.value,
      frame.update.state,
      frame.update.reset,
    );
    snapshotReceivedAt.value = performance.now();
    catchup.value = {
      ...catchup.value,
      done: frame.done,
      saving: frame.done === frame.total,
      excavated: Math.max(
        0,
        frame.update.state.excavated - catchup.value.baselineExcavated,
      ),
    };
    if (frame.update.reset) terrainEpoch.value++;
  }
  if (!frames.length && savedResult) finishPlayback();
  else if (frames.length) schedulePlayback();
}
function schedulePlayback() {
  if (playbackScheduled) return;
  playbackScheduled = true;
  requestAnimationFrame(consumeFrame);
}
export function skipCatchup() {
  if (!catchup.value) return;
  catchup.value = { ...catchup.value, skipped: true };
  frames = [];
  frameBytes = 0;
  visualState.value = null;
  if (savedResult) finishPlayback();
}
watch(
  () => preferences.reducedMotion,
  (reduced) => {
    if (reduced) skipCatchup();
  },
);
function receiveVisual(event: VisualEvent) {
  if (event.kind === "start") {
    frames = [];
    frameBytes = 0;
    savedResult = null;
    liveResult = event.state;
    catchup.value = {
      session: event.session,
      done: 0,
      total: event.total,
      skipped: preferences.reducedMotion || document.hidden,
      saving: false,
      baselineExcavated: event.state.excavated,
      excavated: 0,
    };
    if (!state.value) state.value = event.state;
    visualState.value = catchup.value.skipped ? null : event.state;
    terrainEpoch.value++;
    snapshotReceivedAt.value = performance.now();
  } else if (catchup.value?.session !== event.session) return;
  else if (event.kind === "frame") {
    if (catchup.value.skipped) {
      catchup.value = {
        ...catchup.value,
        done: event.done,
        saving: event.done === event.total,
        excavated: Math.max(
          0,
          event.update.state.excavated - catchup.value.baselineExcavated,
        ),
      };
      return;
    }
    frames.push(event);
    frameBytes += JSON.stringify(event).length * 2;
    while (frameBytes > 16 * 1024 * 1024 && frames.length > 1) {
      const left = frames.shift()!,
        right = frames[0]!;
      const leftSize = JSON.stringify(left).length * 2;
      const rightSize = JSON.stringify(right).length * 2;
      if (!right.update.reset) {
        for (const key of ["chunks", "visible", "revealed"] as const) {
          right.update.state.terrain[key] = {
            ...left.update.state.terrain[key],
            ...right.update.state.terrain[key],
          };
        }
        right.update.state.workings.passages = [
          ...left.update.state.workings.passages,
          ...right.update.state.workings.passages,
        ];
        right.update.state.workings_offset = left.update.state.workings_offset;
        right.update.reset = left.update.reset;
      }
      frameBytes += JSON.stringify(right).length * 2 - leftSize - rightSize;
    }
    if (frameBytes > 16 * 1024 * 1024) consumeFrame();
    else schedulePlayback();
  } else if (event.kind === "saved") {
    savedResult = event.state;
    liveResult = latestWorldSnapshot(event.state, liveResult);
    catchup.value = { ...catchup.value, saving: false };
    if (!frames.length || catchup.value.skipped) finishPlayback();
  } else {
    frames = [];
    frameBytes = 0;
    savedResult = null;
    liveResult = null;
    visualState.value = null;
    catchup.value = null;
    state.value = event.state;
    snapshotReceivedAt.value = performance.now();
    error.value = event.message;
    reconciling.value = false;
    terrainEpoch.value++;
  }
}
export const terrainEpoch = shallowRef(0);
export const error = shallowRef("");
export const native = isTauri();
export const reconciling = shallowRef(native);
export function format(n: number | string) {
  return displayNumber(n, preferences.numbers);
}
export const upgrades = [
  ["slagcrusher", "Slag crusher", "Recover construction aggregate from slag."],
  ["wheelbarrow", "Handcart fleet", "Double crew capacity on branch haulage."],
  ["minecart", "Minecart railway", "Move cargo through deep shafts."],
  ["train", "Powered trains", "Reduce deep cargo transit time."],
  ["survey", "Survey office", "Assign a dedicated prospector."],
  ["supports", "Support workshop", "Support deep branching tunnels."],
  ["pump", "Drainage pumps", "Clear groundwater below 700 m."],
  ["ventilation", "Ventilation plant", "Manage heat below 1,500 m."],
  ["worker", "Recruit crew", "More hands, more progress."],
  ["housing", "Bunkhouse", "Room for four more workers."],
  ["drill", "Powered picks", "Increase excavation work."],
  ["conveyor", "Branch conveyors", "Automate ore flow from active work faces."],
  ["sorter", "Sorting deck", "Separate material faster."],
  ["furnace", "Smelting furnace", "Turn mineral feed into products."],
  ["shaft", "Hoist upgrade", "Increase lift capacity and open deeper access."],
  ["steelworks", "Steelworks", "Iron, coke and lime become steel."],
  ["power", "Power station", "Prepare electrical infrastructure."],
  ["chemical", "Chemical refinery", "Unlock chemical and sulfide feeds."],
  ["electrolytic", "Electrolysis hall", "Unlock aluminium production."],
  ["trace", "Separation hall", "Recover specialist products."],
  [
    "manufacturing",
    "Component workshop",
    "Manufacture alloys and advanced parts.",
  ],
  ["recovery", "Recovery screens", "Recover 3% more mineral content."],
  [
    "capacity",
    "Transfer buffers",
    "Expand every work-face and transfer buffer.",
  ],
  ["reclaimer", "Tailings recovery", "Recover retained mineral content."],
] as const;
export function cost(id: string) {
  return viewState.value?.quotes[id] ?? "0";
}
let lifecycleRegistered = false;
async function backgroundState(background: boolean) {
  try {
    if (background) skipCatchup();
    const next = await invoke<Game>("set_background", { background });
    if (
      !catchup.value &&
      (!state.value ||
        state.value.campaign_id !== next.campaign_id ||
        state.value.ticks <= next.ticks)
    ) {
      state.value = next;
      snapshotReceivedAt.value = performance.now();
    }
  } catch (e) {
    error.value = String(e);
  }
}
export async function start() {
  if (!native) {
    error.value =
      "Open through Tauri to start the Rust simulation. Browser preview shows an illustrative site.";
    return;
  }
  try {
    if (!lifecycleRegistered) {
      await listen<boolean>("mine-reconciling", (event) => {
        reconciling.value = event.payload;
      });
      await listen<string>("mine-lifecycle-error", (event) => {
        error.value = event.payload;
      });
    }
    const channel = new Channel<{ state: Game; reset: boolean }>();
    channel.onmessage = (update) => {
      const g = update.state;
      if (catchup.value) {
        if (liveResult?.campaign_id === g.campaign_id)
          liveResult = mergeWorldUpdate(liveResult, g, update.reset);
        return;
      }
      if (state.value && state.value.campaign_id !== g.campaign_id) return;
      if (update.reset) terrainEpoch.value++;
      state.value = mergeWorldUpdate(state.value, g, update.reset);
      snapshotReceivedAt.value = performance.now();
    };
    const visualChannel = new Channel<VisualEvent>();
    visualChannel.onmessage = receiveVisual;
    const connected = await invoke<Game>("connect", { channel, visualChannel });
    if (
      !catchup.value &&
      (!state.value || state.value.ticks <= connected.ticks)
    ) {
      state.value = connected;
      snapshotReceivedAt.value = performance.now();
    }
    await backgroundState(document.hidden);
    reconciling.value = false;
    if (!lifecycleRegistered) {
      lifecycleRegistered = true;
      document.addEventListener(
        "visibilitychange",
        () => void backgroundState(document.hidden),
      );
      window.addEventListener("pagehide", () => void backgroundState(true));
      window.addEventListener(
        "pageshow",
        () => void backgroundState(document.hidden),
      );
    }
  } catch (e) {
    error.value = String(e);
  }
}
let pending = false;
export async function act(kind: string, target = "", value = 0) {
  if (!state.value || pending || commandsLocked.value) return false;
  pending = true;
  try {
    state.value = await invoke<Game>("command", {
      campaignId: state.value.campaign_id,
      action: { sequence: state.value.last_sequence + 1, kind, target, value },
    });
    snapshotReceivedAt.value = performance.now();
    error.value = "";
    purchaseSound();
    return true;
  } catch (e) {
    error.value = String(e);
    return false;
  } finally {
    pending = false;
  }
}
export async function exportSave() {
  try {
    const data = await invoke<ArrayBuffer | number[]>("export_save");
    const bytes = data instanceof ArrayBuffer ? data : new Uint8Array(data);
    const url = URL.createObjectURL(
      new Blob([bytes], { type: "application/octet-stream" }),
    );
    const a = document.createElement("a");
    a.href = url;
    a.download = "deepwork-save.deepwork";
    a.click();
    URL.revokeObjectURL(url);
  } catch (e) {
    error.value = String(e);
  }
}
export async function importSave(file: File) {
  if (commandsLocked.value) return;
  importing.value = true;
  try {
    const bytes = new Uint8Array(await file.arrayBuffer());
    const first = bytes.find((byte) => ![9, 10, 13, 32].includes(byte));
    const legacyJson =
      file.name.toLowerCase().endsWith(".json") || first === 0x7b;
    const imported = await invoke<Game>("import_save", {
      data: legacyJson ? new TextDecoder().decode(bytes) : bytesToBase64(bytes),
      encoding: legacyJson ? "json" : "base64",
    });
    if (
      !catchup.value &&
      (!state.value ||
        state.value.campaign_id !== imported.campaign_id ||
        state.value.ticks <= imported.ticks)
    ) {
      state.value = imported;
      snapshotReceivedAt.value = performance.now();
    }
    terrainEpoch.value++;
  } catch (e) {
    error.value = String(e);
  } finally {
    importing.value = false;
  }
}

function bytesToBase64(bytes: Uint8Array) {
  let binary = "";
  for (let offset = 0; offset < bytes.length; offset += 0x8000) {
    binary += String.fromCharCode(...bytes.subarray(offset, offset + 0x8000));
  }
  return btoa(binary);
}

export async function resetCampaign(confirmation: string) {
  if (!state.value || pending || commandsLocked.value) return false;
  pending = true;
  try {
    state.value = await invoke<Game>("reset_campaign", {
      confirmation,
      campaignId: state.value.campaign_id,
    });
    snapshotReceivedAt.value = performance.now();
    terrainEpoch.value++;
    error.value = "";
    return true;
  } catch (e) {
    error.value = String(e);
    return false;
  } finally {
    pending = false;
  }
}
