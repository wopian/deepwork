import { shallowRef } from "vue";
import { Channel, invoke, isTauri } from "@tauri-apps/api/core";
import catalogue from "../content/materials.json";
import { purchaseSound } from "./preferences";
export const materials = catalogue;
export interface Game {
  enabled_recipes: string[];
  megaproject: boolean;
  site_discoveries: number;
  version: number;
  seed: number;
  site: number;
  profile: number;
  trace_feed: Record<string, number>;
  ticks: number;
  credits: string;
  workers: number;
  housing: number;
  levels: Record<string, number>;
  research: number;
  ranks: Record<string, number>;
  policy: string;
  priorities: number[];
  reserve: Record<string, number>;
  pinned: string | null;
  shipments: {
    material: number;
    amount: number;
    remaining: number;
    duration: number;
    depth: number;
    mode: string;
  }[];
  crew: Record<string, number>;
  heights: number[];
  terrain: { chunks: Record<string, number[]>; revision: number };
  removed: { x: number; y: number; material: number }[];
  ore: Record<string, number>;
  hauled: Record<string, number>;
  products: Record<string, number>;
  tailings: Record<string, number>;
  slag: number;
  depleted: number;
  lifetime_waste: number;
  excavated: number;
  discoveries: number[];
  contracts: { product: string; amount: number; complete: boolean }[];
  records: {
    site: number;
    profile: number;
    trace_feed: Record<string, number>;
    depth: number;
    research: number;
    excavated: number;
  }[];
  cooldowns: number[];
  boosts: number[];
  last_sequence: number;
  stages: {
    name: string;
    rate: number;
    buffer: number;
    capacity: number;
    blocker: string;
  }[];
  offline: null | {
    elapsed: number;
    effective: number;
    credits: string;
    excavated: number;
  };
  steel_made: boolean;
}
export const state = shallowRef<Game | null>(null);
export const error = shallowRef("");
export const native = isTauri();
export function format(n: number | string) {
  const v = Number(n);
  return new Intl.NumberFormat("en", {
    notation: v >= 10000 ? "compact" : "standard",
    maximumFractionDigits: 1,
  }).format(v);
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
  ["conveyor", "Conveyor line", "Move ore out of the pit.", 75],
  ["sorter", "Sorting deck", "Separate material faster.", 100],
  ["furnace", "Smelting furnace", "Turn mineral feed into products.", 160],
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
  const g = state.value;
  if (!g) return 0;
  const base = upgrades.find((u) => u[0] === id)?.[3] ?? 100;
  const n =
    id === "worker"
      ? g.workers - 3
      : id === "housing"
        ? (g.housing - 8) / 4
        : (g.levels[id] ?? 0);
  return Math.ceil(
    base * Math.pow(id === "worker" ? 1.15 : id === "housing" ? 1.12 : 1.18, n),
  );
}
export async function start() {
  if (!native) {
    error.value =
      "Open through Tauri to start the Rust simulation. Browser preview shows an illustrative site.";
    return;
  }
  try {
    const channel = new Channel<{ state: Game; reset: boolean }>();
    channel.onmessage = (update) => {
      const g = update.state;
      if (!update.reset && state.value?.site === g.site) {
        g.terrain.chunks = {
          ...state.value.terrain.chunks,
          ...g.terrain.chunks,
        };
      }
      state.value = g;
    };
    state.value = await invoke<Game>("connect", { channel });
  } catch (e) {
    error.value = String(e);
  }
}
let pending = false;
export async function act(kind: string, target = "", value = 0) {
  if (!state.value || pending) return;
  pending = true;
  try {
    state.value = await invoke<Game>("command", {
      action: { sequence: state.value.last_sequence + 1, kind, target, value },
    });
    error.value = "";
    purchaseSound();
  } catch (e) {
    error.value = String(e);
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
  } catch (e) {
    error.value = String(e);
  }
}
