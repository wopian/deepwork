import { shallowRef } from "vue";
import { Channel, invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import catalogue from "../content/materials.json";
import { displayNumber } from "./numbers";
import { preferences, purchaseSound } from "./preferences";
export const materials = catalogue;
export interface Game {
  campaign_id: string;
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
  workings_offset: number;
  workings: {
    target: [number, number] | null;
    target_deposit: string | null;
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
    { machine_percent: number; line_percent: number }
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
export const terrainEpoch = shallowRef(0);
export const error = shallowRef("");
export const native = isTauri();
export const reconciling = shallowRef(native);
export function format(n: number | string) {
  return displayNumber(n, preferences.numbers);
}
export const upgrades = [
  [
    "slagcrusher",
    "Slag crusher",
    "Recover construction aggregate from slag.",
    100,
  ],
  ["wheelbarrow", "Wheelbarrow fleet", "Faster short-distance hauling.", 100],
  ["minecart", "Minecart railway", "Move cargo through deep shafts.", 100],
  ["train", "Powered trains", "Reduce deep cargo transit time.", 100],
  ["survey", "Survey office", "Assign a dedicated prospector.", 100],
  ["supports", "Support workshop", "Support deep branching tunnels.", 100],
  ["pump", "Drainage pumps", "Clear groundwater below 700 m.", 100],
  ["ventilation", "Ventilation plant", "Manage heat below 1,500 m.", 100],
  ["worker", "Recruit minion", "More hands, more progress.", 25],
  ["housing", "Bunkhouse", "Room for four more workers.", 90],
  ["drill", "Powered picks", "Increase excavation work.", 100],
  ["conveyor", "Conveyor line", "Move ore out of the pit.", 450],
  ["sorter", "Sorting deck", "Separate material faster.", 100],
  ["furnace", "Smelting furnace", "Turn mineral feed into products.", 800],
  ["shaft", "Shaft & lift", "Open deeper excavation fronts.", 600],
  ["steelworks", "Steelworks", "Iron, coke and lime become steel.", 450],
  ["power", "Power station", "Prepare electrical infrastructure.", 900],
  ["chemical", "Chemical refinery", "Unlock chemical and sulfide feeds.", 1500],
  ["electrolytic", "Electrolysis hall", "Unlock aluminium production.", 2200],
  ["trace", "Separation hall", "Recover specialist products.", 3500],
  [
    "manufacturing",
    "Component workshop",
    "Manufacture alloys and advanced parts.",
    100,
  ],
  ["recovery", "Recovery screens", "Recover 3% more mineral content.", 100],
  ["capacity", "Loading depot", "Expand material buffers.", 100],
  ["reclaimer", "Tailings recovery", "Recover retained mineral content.", 100],
] as const;
export function cost(id: string) {
  return state.value?.quotes[id] ?? "0";
}
let lifecycleRegistered = false;
async function backgroundState(background: boolean) {
  try {
    state.value = await invoke<Game>("set_background", { background });
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
      if (state.value && state.value.campaign_id !== g.campaign_id) return;
      if (update.reset) terrainEpoch.value++;
      if (!update.reset && state.value?.site === g.site) {
        if (g.workings_offset > 0) {
          g.workings.passages = [
            ...state.value.workings.passages.slice(0, g.workings_offset),
            ...g.workings.passages,
          ];
        }
        for (const key of ["chunks", "revealed", "visible"] as const) {
          g.terrain[key] = { ...state.value.terrain[key], ...g.terrain[key] };
        }
      }
      state.value = g;
    };
    state.value = await invoke<Game>("connect", { channel });
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
  if (!state.value || pending || reconciling.value) return false;
  pending = true;
  try {
    state.value = await invoke<Game>("command", {
      campaignId: state.value.campaign_id,
      action: { sequence: state.value.last_sequence + 1, kind, target, value },
    });
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
    const data = await invoke<string>("export_save");
    const url = URL.createObjectURL(
      new Blob([data], { type: "application/json" }),
    );
    const a = document.createElement("a");
    a.href = url;
    a.download = "deepwork-save.json";
    a.click();
    URL.revokeObjectURL(url);
  } catch (e) {
    error.value = String(e);
  }
}
export async function importSave(file: File) {
  try {
    state.value = await invoke<Game>("import_save", {
      data: await file.text(),
    });
    terrainEpoch.value++;
  } catch (e) {
    error.value = String(e);
  }
}

export async function resetCampaign(confirmation: string) {
  if (!state.value || pending) return false;
  pending = true;
  try {
    state.value = await invoke<Game>("reset_campaign", {
      confirmation,
      campaignId: state.value.campaign_id,
    });
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
