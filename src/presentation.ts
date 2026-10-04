import type { Game } from "./game";
import requirements from "../content/upgrades.json";
import pacing from "../content/pacing.json";
import catalogue from "../content/materials.json";
import { CELLS_PER_METRE, RESOURCE_UNIT } from "./geometry";

export type PanelName =
  | "Equipment"
  | "Processing"
  | "Crew"
  | "Production"
  | "Logistics"
  | "Industry"
  | "Minerals"
  | "Contracts"
  | "Headquarters"
  | "Records"
  | "Settings";
export type DockName = "Build" | "Crew" | "Industry" | "Minerals" | "More";
export const navigation: Record<DockName, PanelName[]> = {
  Build: ["Equipment", "Processing"],
  Crew: ["Crew"],
  Industry: ["Production", "Logistics", "Industry"],
  Minerals: ["Minerals"],
  More: ["Contracts", "Headquarters", "Records", "Settings"],
};
export function dockFor(panel: PanelName): DockName {
  return (Object.keys(navigation) as DockName[]).find((name) =>
    navigation[name].includes(panel),
  )!;
}
export interface Goal {
  id: string;
  title: string;
  detail: string;
  panel: PanelName;
  progress: number;
  target?: string;
}
const names: Record<string, string> = {
  furnace: "Refine your first iron",
  conveyor: "Mechanise hauling",
  sorter: "Sort your ore",
  steelworks: "Produce steel",
  shaft: "Extend the mine lift",
  supports: "Strengthen tunnel supports",
  power: "Power your industry",
  manufacturing: "Build a component workshop",
  chemical: "Open chemical processing",
  pump: "Drain deeper workings",
  electrolytic: "Refine advanced metals",
  ventilation: "Ventilate deep tunnels",
  trace: "Separate rare earths",
};
function upgradeGoal(game: Game, target: string): Goal {
  const requirement = requirements.find((item) => item.id === target);
  if (requirement?.requires && !game.levels[requirement.requires])
    return upgradeGoal(game, requirement.requires);
  const blocker = game.purchase_blockers[target] ?? "";
  const fractions = [
    Math.min(
      1,
      Number(game.credits) / Math.max(1, Number(game.quotes[target] ?? 1)),
    ),
  ];
  if (requirement) {
    for (const [product, amount] of Object.entries(requirement.inputs)) {
      // Authored upgrade costs use thousandths; inventory uses RESOURCE_UNIT.
      fractions.push(
        Math.min(
          1,
          (game.products[product] ?? 0) /
            ((Number(amount) * RESOURCE_UNIT) / 1000),
        ),
      );
    }
    if ("research_points" in requirement && requirement.research_points) {
      fractions.push(
        Math.min(1, game.research_invested / requirement.research_points),
      );
    }
  }
  return {
    id: `upgrade:${target}`,
    target,
    title: names[target] ?? `Upgrade ${target.replaceAll("_", " ")}`,
    detail: blocker || "Ready to build. Open equipment to review your upgrade.",
    panel: /research/i.test(blocker) ? "Headquarters" : "Equipment",
    progress: Math.max(0, Math.min(...fractions)),
  };
}
/** Read-only guidance. Never creates a purchase, reserve, or crew order. */
export function nextGoal(game: Game | null): Goal | null {
  if (!game) return null;
  const pinned = game.pinned ?? game.build_queue[0];
  if (pinned) return upgradeGoal(game, pinned);
  for (const target of [
    "furnace",
    "conveyor",
    "sorter",
    "steelworks",
    "shaft",
    "supports",
  ]) {
    if (!game.levels[target]) return upgradeGoal(game, target);
  }
  if (!game.steel_made)
    return {
      id: "steel",
      title: "Produce your first steel",
      detail: "Keep iron, coke and lime flowing to the steelworks.",
      panel: "Industry",
      progress: 0,
    };
  const depth = Math.max(0, ...Object.values(game.heights)) / CELLS_PER_METRE;
  if (depth < pacing.specialisation_depth)
    return {
      id: "depth",
      title: `Reach ${pacing.specialisation_depth} m`,
      detail:
        game.stages.find((stage) => stage.name === "Digging")?.blocker ??
        "Follow the crew as they open deeper workings.",
      panel: "Crew",
      progress: depth / pacing.specialisation_depth,
    };
  if (!game.specialisation)
    return {
      id: "specialisation",
      title: "Choose your site's speciality",
      detail: "Steel and 300 m unlock a permanent choice for this site.",
      panel: "Contracts",
      progress: 1,
    };
  for (const target of [
    "power",
    "manufacturing",
    "chemical",
    "pump",
    "electrolytic",
    "ventilation",
    "trace",
  ]) {
    if (!game.levels[target]) return upgradeGoal(game, target);
  }
  return {
    id: "headquarters",
    title: game.megaproject
      ? "Build your next legacy"
      : "Complete your headquarters",
    detail: game.megaproject
      ? "Explore another site or keep expanding this mine."
      : "Invest research and deliver advanced components at headquarters.",
    panel: "Headquarters",
    progress: game.megaproject
      ? 1
      : Math.min(
          1,
          game.research_invested / pacing.headquarters_research,
          ...[
            "advanced_structures",
            "precision_controls",
            "magnets",
            "batteries",
          ].map(
            (product) => (game.products[product] ?? 0) / (10 * RESOURCE_UNIT),
          ),
        ),
  };
}
export interface MineNotice {
  id: string;
  title: string;
  detail: string;
  kind: "mineral" | "milestone";
}
/** A baseline is required, so loading a save never celebrates old discoveries. */
export class NoticeTracker {
  private campaign = "";
  private discoveries = new Set<number>();
  private milestones = new Set<string>();
  private baseline = false;
  update(game: Game | null, suppressed = false): MineNotice[] {
    if (!game) return [];
    const fresh = !this.baseline || this.campaign !== game.campaign_id;
    const notices: MineNotice[] = [];
    if (!fresh && !suppressed) {
      for (const id of game.discoveries)
        if (!this.discoveries.has(id)) {
          const mineral = catalogue.find((item) => item.id === id);
          if (mineral)
            notices.push({
              id: `${game.campaign_id}:mineral:${id}`,
              title: `${mineral.name} discovered`,
              detail: "A new mineral joins your field guide.",
              kind: "mineral",
            });
        }
      for (const milestone of game.milestones)
        if (!this.milestones.has(milestone))
          notices.push({
            id: `${game.campaign_id}:milestone:${milestone}`,
            title: milestone,
            detail: "A new chapter in your mine's story.",
            kind: "milestone",
          });
    }
    this.campaign = game.campaign_id;
    this.discoveries = new Set(game.discoveries);
    this.milestones = new Set(game.milestones);
    this.baseline = true;
    return notices;
  }
}
