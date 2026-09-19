<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import World from "./World.vue";
import ProcessingPanel from "./ProcessingPanel.vue";
import { RESOURCE_UNIT, CELLS_PER_METRE } from "./geometry";
import upgradeRequirements from "../content/upgrades.json";
import profiles from "../content/sites.json";
import recipes from "../content/recipes.json";
import traces from "../content/traces.json";
import { resourceQuantity } from "./numbers";
import pacing from "../content/pacing.json";
import mining from "../content/mining.json";
import { preferences } from "./preferences";
import {
  state,
  reconciling,
  error,
  materials,
  upgrades,
  format,
  cost,
  act,
  start,
  exportSave,
  importSave,
  resetCampaign,
} from "./game";
const discoveredMaterials = computed(() =>
  materials.filter((m) => state.value?.discoveries.includes(m.id)),
);
const recipeCards = recipes.map((recipe) => ({
  ...recipe,
  buildingName:
    upgrades.find(([id]) => id === recipe.building)?.[1] ?? recipe.building,
  sources: [
    ...new Set(
      traces
        .filter(
          (rule) =>
            recipe.id.startsWith("separate_") && rule.output === recipe.output,
        )
        .map((rule) => materials[rule.feed]!.name),
    ),
  ],
}));
const headquartersBenefits = (branch: string) =>
  (
    pacing.headquarters_starting as Record<
      string,
      { rank: number; upgrades: string[] }[]
    >
  )[branch] ?? [];
const showReset = ref(false);
const resetText = ref("");
async function confirmReset() {
  if (await resetCampaign(resetText.value)) {
    showReset.value = false;
    resetText.value = "";
  }
}
const batchTotal = (batches: { amount: number }[]) =>
  batches.reduce((total, batch) => total + batch.amount, 0);
const cargoTotal = (cargo: Record<string, number>) =>
  Object.values(cargo).reduce((sum, quantity) => sum + quantity, 0);
const materialUpgrades = upgradeRequirements.filter(
  (u) => Object.keys(u.inputs).length,
);
const productCatalogue = [
  ...new Set([
    ...materials.map((m) => m.product),
    ...recipes.map((r) => r.output),
    "alumina",
  ]),
].sort();
const selectedSite = ref(0);
const selectedChallenge = ref("");
function sectionPath(section: number[]) {
  return section
    .map((open, i) => (open ? `M${i % 64} ${Math.floor(i / 64)}h1v1h-1z` : ""))
    .join(" ");
}
const upgradeOrder = [
  "worker",
  "housing",
  "drill",
  "wheelbarrow",
  "conveyor",
  "sorter",
  "furnace",
  "steelworks",
  "shaft",
  "supports",
  "minecart",
  "power",
  "manufacturing",
  "chemical",
  "pump",
  "electrolytic",
  "ventilation",
  "train",
  "trace",
  "survey",
  "recovery",
  "capacity",
  "reclaimer",
  "slagcrusher",
];
const orderedUpgrades = [...upgrades].sort(
  (a, b) => upgradeOrder.indexOf(a[0]) - upgradeOrder.indexOf(b[0]),
);
const tab = ref("Operations");
const panel = ref("");
const panelNames = [
  "Equipment",
  "Processing",
  "Crew",
  "Logistics",
  "Production",
  "Contracts",
];
const constraint = computed(() => {
  const stages = state.value?.stages ?? [];
  return (
    [...stages]
      .reverse()
      .find((s) =>
        /full|required|limited|locked|No walkable/i.test(s.blocker),
      ) ?? stages[0]
  );
});
function openPanel(name: string) {
  panel.value = panel.value === name ? "" : name;
}
const query = ref("");
const showRetire = ref(false);
async function retirementPreview() {
  if (await act("retirement_preview")) showRetire.value = true;
}
async function retirementConfirm() {
  if (await act("retire", selectedChallenge.value, selectedSite.value))
    showRetire.value = false;
}
async function retirementCancel() {
  if (await act("cancel_retirement")) showRetire.value = false;
}
const dismissedOffline = ref("");
const depth = computed(() =>
  Math.floor(
    Object.values(state.value?.heights ?? {}).reduce(
      (max, h) => Math.max(max, h),
      0,
    ) / CELLS_PER_METRE,
  ),
);
const filtered = computed(() =>
  materials.filter((m) =>
    (m.name + " " + m.product)
      .toLowerCase()
      .includes(query.value.toLowerCase()),
  ),
);
const ready = computed(() => !!state.value?.steel_made && depth.value >= 300);
onMounted(start);
</script>
<template>
  <div
    class="shell app-frame"
    :class="{ 'mine-first': tab === 'Operations' }"
    :style="{ zoom: preferences.uiScale }"
  >
    <header>
      <a class="brand" href="#"
        ><span class="brand-mark">▧</span
        ><span>DEEPWORK<small>MINING COMPANY</small></span></a
      >
      <nav>
        <button
          v-for="name in [
            'Operations',
            'Minerals',
            'Industry',
            'Headquarters',
            'Records',
            'Settings',
          ]"
          :key="name"
          :class="{ active: tab === name }"
          @click="tab = name"
        >
          {{ name }}
        </button>
      </nav>
      <div class="site-tag">
        <i /> SITE {{ String(state?.site ?? 1).padStart(2, "0") }}
        <span> · {{ profiles[state?.profile ?? 0].name.toUpperCase() }}</span>
      </div>
    </header>
    <div class="notice" v-if="state?.requires_reset" role="status">
      Older campaign requires a fresh start. Export remains available in
      Records; reset archives the original save.
      <button
        @click="
          showReset = true;
          resetText = '';
        "
      >
        Start fresh
      </button>
    </div>
    <div class="notice" v-if="reconciling" role="status">
      Catching up your mine… Progress is being saved before play resumes.
    </div>
    <div class="notice" v-if="error" role="status">
      {{ error }}<button @click="error = ''" aria-label="Dismiss">×</button>
    </div>
    <details
      class="offline"
      v-if="state?.offline?.effective && state.offline.id !== dismissedOffline"
    >
      <summary>
        <span>While away · +{{ format(state.offline.credits) }} credits</span
        ><button
          @click.stop.prevent="dismissedOffline = state?.offline?.id ?? ''"
          aria-label="Dismiss return report"
        >
          ×
        </button>
      </summary>
      <p>
        {{ format(state.offline.excavated) }} cells excavated ·
        {{
          state.offline.effective < 60
            ? state.offline.effective + " simulated seconds"
            : format(state.offline.effective / 60) + " simulated minutes"
        }}
        credited at 50% speed from
        {{
          state.offline.elapsed < 60
            ? state.offline.elapsed + " seconds"
            : format(state.offline.elapsed / 60) + " minutes"
        }}
        away.
      </p>
      <p v-if="state.offline.capped">
        {{ format(state.offline.capped / 3600) }} hours beyond eight-hour cap.
      </p>
      <p v-if="state.offline.discoveries.length">
        Discovered
        {{
          state.offline.discoveries
            .map((id: number) => materials[id].name)
            .join(", ")
        }}.
      </p>
      <p v-for="blocker in state.offline.blockers">{{ blocker }}</p>
    </details>
    <p class="notice" v-if="state?.challenge">
      SITE CHALLENGE ·
      {{
        state.challenge === "hard_rock"
          ? "Hard rock: 50% more excavation work."
          : "Long haul: 50% longer underground routes."
      }}
      Steel + 300 m awards 5 extra retirement research.
    </p>
    <div class="page-heading">
      <div class="eyebrow">SHAFT OPERATIONS</div>
      <div class="title-row">
        <h1>
          {{
            tab === "Operations"
              ? "Deepwork"
              : tab === "Minerals"
                ? "Mineral collection"
                : tab === "Industry"
                  ? "Mine industry"
                  : tab === "Headquarters"
                    ? "Headquarters"
                    : tab === "Settings"
                      ? "Settings"
                      : "Mine records"
          }}
        </h1>
        <div class="balance">
          <small>AVAILABLE CREDITS</small
          ><strong>◈ {{ format(state?.credits ?? 0) }}</strong>
        </div>
      </div>
    </div>
    <template v-if="tab === 'Operations'">
      <div class="mine-hud">
        <span class="hud-credits">◈ {{ format(state?.credits ?? 0) }}</span>
        <span
          ><strong>{{ depth }} m</strong> depth</span
        >
        <button @click="openPanel('Crew')">
          <strong>{{ state?.workers ?? 3 }}</strong> crew
        </button>
        <button class="constraint" @click="openPanel('Production')">
          {{ constraint?.name ?? "Production" }} ·
          {{ constraint?.blocker ?? "Crew preparing access" }}
        </button>
      </div>
      <div class="operation-grid">
        <section class="mine-panel">
          <div class="panel-heading">
            <h2>THE WORKSITE</h2>
            <span
              ><i />
              {{ state ? "CREW AT WORK" : "AWAITING NATIVE ENGINE" }}</span
            >
          </div>
          <World @inspect="panel = $event" />
          <div class="opening-guide" v-if="state && !state.levels.furnace">
            <strong>Next · refine iron</strong>
            <button
              @click="act('buy', 'furnace')"
              :disabled="!!state.purchase_blockers.furnace"
            >
              Build furnace · {{ state.quotes.furnace }} credits
            </button>
            <span v-if="state.purchase_blockers.furnace">{{
              state.purchase_blockers.furnace
            }}</span>
          </div>
        </section>
        <aside
          v-if="panel"
          class="context-panel"
          :aria-label="panel + ' controls'"
          @keydown.esc="panel = ''"
        >
          <div class="context-heading">
            <h2>{{ panel }}</h2>
            <button @click="panel = ''" aria-label="Close panel">✕</button>
          </div>
          <div class="context-body">
            <div v-if="panel === 'Equipment'">
              <div class="panel-heading">
                <h2>CREW & EQUIPMENT</h2>
                <span>UPGRADES</span>
              </div>
              <p class="crew-roster">
                Capacity estimates assume steady feed and completed
                construction.
              </p>
              <div class="upgrade-list">
                <div
                  v-for="u in orderedUpgrades"
                  :key="u[0]"
                  class="upgrade-row"
                >
                  <button
                    class="upgrade"
                    :disabled="!state || !!state.purchase_blockers[u[0]]"
                    @click="act('buy', u[0])"
                  >
                    <div class="upgrade-icon">
                      {{
                        u[0] === "worker"
                          ? "♟"
                          : u[0] === "furnace"
                            ? "♨"
                            : "▥"
                      }}
                    </div>
                    <div>
                      <strong>{{ u[1] }}</strong>
                      <p>{{ u[2] }}</p>
                      <small>LEVEL {{ state?.levels[u[0]] ?? 0 }}</small>
                    </div>
                    <b>◈ {{ format(cost(u[0])) }}</b>
                    <small
                      class="upgrade-blocker"
                      v-if="state?.upgrade_previews[u[0]]"
                      >Machine
                      {{
                        format(state.upgrade_previews[u[0]].machine_percent)
                      }}% · feed line ~{{
                        format(state.upgrade_previews[u[0]].line_percent)
                      }}%</small
                    >
                    <small
                      class="upgrade-blocker"
                      v-if="
                        state?.upgrade_previews[u[0]]?.lift_depth_after != null
                      "
                    >
                      Lift reach after upgrade:
                      {{
                        format(state.upgrade_previews[u[0]].lift_depth_after!)
                      }}
                      m. Current support, pump and ventilation equipment permits
                      {{
                        format(state.upgrade_previews[u[0]].access_depth_after!)
                      }}
                      m.
                    </small>
                    <small
                      class="upgrade-blocker"
                      v-if="state?.purchase_blockers[u[0]]"
                      >{{ state.purchase_blockers[u[0]] }}</small
                    >
                  </button>
                  <button
                    class="pin-upgrade"
                    :aria-pressed="state?.pinned === u[0]"
                    :disabled="!state"
                    @click="act('pin', u[0])"
                  >
                    {{
                      state?.pinned === u[0]
                        ? "Release reserved materials"
                        : "Save materials for this"
                    }}
                  </button>
                </div>
              </div>
            </div>
            <ProcessingPanel v-if="panel === 'Processing'" />
            <div v-if="panel === 'Crew'">
              <div class="selected-order" v-if="state?.selected_vein">
                <strong>Selected vein</strong>
                <p>
                  {{ state.selected_vein.known_cells }} revealed cells remain.
                  White edges mark your order.
                </p>
                <p>
                  Crews prioritize reachable ore. Rock needed for floors and
                  support columns stays in place.
                </p>
                <p v-if="!state.selected_vein.known_cells">
                  No revealed ore remains. Prospect further or choose another
                  vein.
                </p>
                <button @click="act('clear_vein')">Clear vein order</button>
              </div>
              <p class="crew-roster" v-if="state">
                <span v-for="(count, role) in state.crew" :key="role"
                  >{{ count }} {{ role }}</span
                >
              </p>
              <label class="crew-roster" v-if="state"
                >Crew priority
                <select
                  :value="state.crew_priority || 'balanced'"
                  @change="
                    act(
                      'crew_priority',
                      ($event.target as HTMLSelectElement).value,
                    )
                  "
                >
                  <option value="engineering">
                    Engineering · needs support workshop
                  </option>
                  <option value="prospecting">
                    Prospecting · needs survey office
                  </option>
                  <option value="balanced">Balanced</option>
                  <option value="digging">Digging</option>
                  <option value="hauling">Hauling</option>
                  <option value="refining">Refining · needs furnace</option>
                  <option value="reclaiming">
                    Reclamation · needs reclaimer
                  </option>
                </select>
                <span
                  >Shift spare diggers to the bottleneck. One digger and one
                  hauler always remain.</span
                >
              </label>
              <p class="crew-roster" v-if="state">
                {{
                  state.workings.status ||
                  "Supports follow commissioned passages."
                }}
                <template v-if="state.workings.section?.support_work">
                  · Local support construction
                  {{
                    Math.min(
                      100,
                      Math.floor(
                        (100 * state.workings.section.support_work) /
                          mining.support_work,
                      ),
                    )
                  }}%
                </template>
              </p>
              <div class="policy">
                <span
                  >EXCAVATION STRATEGY
                  <small
                    v-if="state?.site === 1 && depth < pacing.tactics_depth"
                    >{{ pacing.tactics_depth }} M UNLOCK</small
                  ></span
                ><button
                  v-for="[id, label] in [
                    ['bulk', 'Bulk excavation'],
                    ['vein', 'Follow veins'],
                    ['depth', 'Go deeper'],
                  ]"
                  :class="{ selected: state?.policy === id }"
                  :disabled="
                    !state || (state.site === 1 && depth < pacing.tactics_depth)
                  "
                  @click="act('policy', id)"
                >
                  {{ label }}
                </button>
              </div>
            </div>
            <div v-if="panel === 'Logistics'">
              <label class="crew-roster" v-if="state">
                Cargo scheduling
                <select
                  :value="state.cargo_policy || 'balanced'"
                  @change="
                    act(
                      'cargo_policy',
                      ($event.target as HTMLSelectElement).value,
                    )
                  "
                >
                  <option value="balanced">Balanced cargo</option>
                  <option value="preferred">Preferred minerals first</option>
                </select>
                <span
                  >Hauling and sorting favour your three mineral priorities.
                  Every fifth slot serves other cargo.</span
                >
              </label>
              <div v-if="state" class="transport-panel">
                <h3>Buffers & express service</h3>
                <div
                  v-for="(station, index) in state.transport.stations"
                  :key="station.id"
                  class="transport-station"
                >
                  <strong>{{ station.name }}</strong>
                  <span
                    >{{ format(cargoTotal(station.cargo) / RESOURCE_UNIT) }} /
                    {{ format(station.capacity / RESOURCE_UNIT) }} units</span
                  >
                  <span
                    >In {{ format(station.incoming / RESOURCE_UNIT) }} · out
                    {{ format(station.outgoing / RESOURCE_UNIT) }} this
                    second</span
                  >
                  <button
                    :disabled="
                      station.level >= 50 ||
                      BigInt(state.credits) < BigInt(station.quote)
                    "
                    @click="act('buffer', station.id)"
                  >
                    Buffer + · {{ format(station.quote) }} credits
                  </button>
                  <label
                    ><input
                      type="checkbox"
                      :checked="station.preferred"
                      @change="
                        act(
                          'station_priority',
                          station.id,
                          ($event.target as HTMLInputElement).checked ? 1 : 0,
                        )
                      "
                    />
                    Prefer selected minerals</label
                  >
                  <template v-if="index < state.transport.segments.length">
                    <span
                      >{{ state.transport.segments[index]!.name }} ·
                      {{
                        Math.round(
                          state.transport.segments[index]!.utilisation * 100,
                        )
                      }}% loading ·
                      {{
                        state.transport.segments[index]!.blocked
                          ? state.transport.segments[index]!.blocker
                          : "Flowing"
                      }}</span
                    >
                    <span class="transport-cargo">
                      In transit
                      {{
                        format(
                          batchTotal(state.transport.segments[index]!.batches) /
                            RESOURCE_UNIT,
                        )
                      }}
                      /
                      {{
                        format(
                          state.transport.segments[index]!.capacity /
                            RESOURCE_UNIT,
                        )
                      }}
                      units
                    </span>
                    <span>
                      Nominal cycle
                      {{
                        format(
                          state.transport.segments[index]!.duration_ms / 1000,
                        )
                      }}
                      s ·
                      {{
                        format(
                          state.transport.segments[index]!.rate / RESOURCE_UNIT,
                        )
                      }}
                      units/s ·
                      {{ state.transport.segments[index]!.demand }} power
                    </span>
                    <button
                      :class="{ selected: state.transport.express === index }"
                      @click="act('express', '', index)"
                    >
                      {{
                        state.transport.express === index
                          ? "Express selected"
                          : "Select express route"
                      }}
                    </button>
                  </template>
                </div>
              </div>
            </div>
            <div v-if="panel === 'Production'">
              <div class="pipeline">
                <div
                  v-for="(s, i) in state?.stages.length
                    ? state.stages
                    : [
                        'Digging',
                        'Hauling',
                        'Sorting',
                        'Refining',
                        'Dispatch',
                      ].map((name) => ({
                        name,
                        rate: 0,
                        buffer: 0,
                        capacity: 20000,
                        blocker: 'Awaiting crew',
                      }))"
                  :key="s.name"
                >
                  <small>0{{ i + 1 }} / {{ s.name.toUpperCase() }}</small
                  ><strong>{{ format(s.rate) }} <em>/s</em></strong>
                  <div class="meter">
                    <span
                      :style="{
                        width:
                          Math.min(100, (s.buffer / s.capacity) * 100) + '%',
                      }"
                    />
                  </div>
                  <p>{{ s.blocker }}</p>
                </div>
              </div>
            </div>
            <div v-if="panel === 'Contracts'">
              <div class="bottom-grid">
                <section class="card specialisations">
                  <div class="panel-heading">
                    <h2>SITE SPECIALISATION</h2>
                    <span>{{
                      state?.specialisation ??
                      `STEEL + ${pacing.specialisation_depth} M`
                    }}</span>
                  </div>
                  <p>Choose once per site. New sites offer a fresh choice.</p>
                  <div class="abilities specialisation-options">
                    <button
                      v-for="[id, label, detail] in [
                        [
                          'bulk',
                          'Bulk extraction',
                          '+30% digging; recovery −5 percentage points.',
                        ],
                        [
                          'precision',
                          'Precision refining',
                          'Recovery +10 points (95% cap); −20% digging.',
                        ],
                        [
                          'reclamation',
                          'Reclamation',
                          '3× tailings and slag recovery; −15% primary refining.',
                        ],
                      ]"
                      :class="{ selected: state?.specialisation === id }"
                      :disabled="
                        !state ||
                        depth < pacing.specialisation_depth ||
                        !state.steel_made ||
                        !!state.specialisation
                      "
                      @click="act('specialise', id)"
                    >
                      <strong>{{ label }}</strong
                      ><small>{{ detail }}</small>
                    </button>
                  </div>
                </section>
                <section class="card contracts">
                  <div class="panel-heading">
                    <h2>OUTGOING ORDERS</h2>
                    <span>25% PREMIUM · NO DEADLINES</span>
                  </div>
                  <div v-for="(c, i) in state?.contracts ?? []">
                    <span
                      >{{ c.product }}
                      <small
                        >{{
                          format(
                            (state?.products[c.product] ?? 0) / RESOURCE_UNIT,
                          )
                        }}
                        / {{ c.amount / RESOURCE_UNIT }} units</small
                      ></span
                    ><button
                      :disabled="
                        !c.complete &&
                        (state?.products[c.product] ?? 0) < c.amount
                      "
                      @click="
                        act(c.complete ? 'new_contract' : 'contract', '', i)
                      "
                    >
                      {{ c.complete ? "New order →" : "Deliver →" }}
                    </button>
                  </div>
                  <p v-if="!state">
                    Delivery contracts appear when native game starts.
                  </p>
                </section>
              </div>
            </div>
          </div>
        </aside>
      </div>
      <nav class="worksite-dock" aria-label="Worksite controls">
        <button
          v-for="name in panelNames"
          :key="name"
          :aria-pressed="panel === name"
          @click="openPanel(name)"
        >
          {{ name }}
        </button>
      </nav>
    </template>
    <section v-else-if="tab === 'Minerals'" class="management-view catalogue">
      <div class="management-grid">
        <section class="workshop-panel is-wide">
          <div class="panel-heading">
            <h2>FIELD GUIDE · {{ materials.length }} FEEDS</h2>
            <input
              v-model="query"
              placeholder="Find a mineral…"
              aria-label="Search minerals"
            />
          </div>
          <p class="panel-copy">
            Real mineral identities. Simplified game processing. Select up to
            three excavation priorities. Cargo scheduling can favour the same
            minerals.
          </p>
          <div class="mineral-grid">
            <button
              v-for="m in filtered"
              :key="m.id"
              :class="{ selected: state?.priorities.includes(m.id) }"
              :disabled="!state"
              @click="act('priority', '', m.id)"
            >
              <span class="mineral-swatch" :style="{ background: m.color }"
                >▨</span
              ><strong
                >{{ m.name
                }}<small
                  >{{ m.family }} → {{ m.product.replaceAll("_", " ") }}</small
                ></strong
              ><b>{{
                state?.discoveries.includes(m.id) ? "FOUND" : "T" + m.tier
              }}</b>
            </button>
          </div>
        </section>
        <section class="workshop-panel is-wide">
          <div class="panel-heading">
            <h2>REFINED COLLECTION</h2>
            <span
              >{{ state?.collection.length ?? 0 }} /
              {{ productCatalogue.length }}</span
            >
          </div>
          <div class="collection-grid">
            <span
              v-for="product in productCatalogue"
              :key="product"
              :class="{ found: state?.collection.includes(product) }"
              >{{ state?.collection.includes(product) ? "◆" : "◇" }}
              {{ product.replaceAll("_", " ") }}</span
            >
          </div>
        </section>
      </div>
    </section>
    <section v-else-if="tab === 'Industry'" class="management-view">
      <div class="management-grid">
        <section class="workshop-panel is-wide">
          <div class="panel-heading">
            <h2>PROCESSING MODULES</h2>
            <span>AUTOMATIC INPUT RESERVES</span>
          </div>
          <p class="panel-copy">
            Toggle production recipes. Active modules retain eight units of each
            input before selling surplus. Steel and aluminium start enabled;
            pause them to route shared feed into other products.
          </p>
          <div class="mineral-grid">
            <button
              v-for="r in recipeCards"
              :disabled="!state"
              :class="{
                selected:
                  !state?.paused_recipes.includes(r.id) &&
                  (state?.enabled_recipes.includes(r.id) ||
                    ['steel', 'aluminium'].includes(r.id)),
              }"
              @click="act('recipe', r.id)"
            >
              <strong
                >{{ r.output.replaceAll("_", " ")
                }}<small>{{
                  Object.entries(r.inputs)
                    .map(([p, n]) => `${n} ${p.replaceAll("_", " ")}`)
                    .join(" + ")
                }}</small
                ><small>{{ r.buildingName }}</small>
                <small v-if="r.sources.length"
                  >Recover from {{ r.sources.join(" or ") }} processing.</small
                ></strong
              >
            </button>
          </div>
        </section>
        <section class="workshop-panel">
          <div class="panel-heading">
            <h2>RECOVERABLE PROCESS RESIDUES</h2>
            <span>TRACE FEED</span>
          </div>
          <div class="mineral-grid compact-grid">
            <div
              v-for="(qty, name) in state?.trace_feed"
              :key="name"
              class="inventory"
            >
              {{ name.replaceAll("_", " ") }} ·
              {{ resourceQuantity(qty) }} units
            </div>
          </div>
        </section>
        <section class="workshop-panel">
          <div class="panel-heading">
            <h2>NEXT EQUIPMENT PURCHASE</h2>
            <span>MATERIAL RESERVE</span>
          </div>
          <p class="panel-copy">
            Pin an upgrade to reserve its material cost and foundation steel for
            upcoming infrastructure. Pin again to release these reserves.
          </p>
          <div class="mineral-grid compact-grid">
            <button
              v-for="upgrade in materialUpgrades"
              :class="{ selected: state?.pinned === upgrade.id }"
              @click="act('pin', upgrade.id)"
            >
              <strong
                >{{ upgrade.id
                }}<small>{{
                  Object.entries(upgrade.inputs)
                    .map(([name, n]) => `${Number(n) / 1000} ${name}`)
                    .join(" + ")
                }}</small
                ><small>Requires {{ upgrade.requires }}</small
                ><small v-if="upgrade.research_points"
                  >{{ upgrade.research_points }} headquarters research
                  invested</small
                ></strong
              >
            </button>
          </div>
        </section>
        <section class="workshop-panel is-wide">
          <div class="panel-heading">
            <h2>RESERVED FEED WAREHOUSE</h2>
            <span v-if="state"
              >{{ format(cargoTotal(state.raw_stock) / RESOURCE_UNIT) }} /
              {{ format(state.raw_stock_capacity / RESOURCE_UNIT) }} UNITS</span
            >
          </div>
          <p v-if="state" class="panel-copy">
            Reserved feed waits for its processing module. Loading depot
            upgrades add space.
          </p>
          <div class="mineral-grid">
            <div v-for="m in discoveredMaterials" :key="m.id" class="inventory">
              <strong
                >{{ m.name }} ·
                {{ format((state?.raw_stock[m.id] ?? 0) / RESOURCE_UNIT) }}
                stored</strong
              >
              <button @click="act('reserve', m.product, 4 * RESOURCE_UNIT)">
                Reserve feed + 4 product
              </button>
              <button @click="act('reserve', m.product, 0)">
                Release manual reserve
              </button>
            </div>
          </div>
        </section>
        <section class="workshop-panel is-wide">
          <div class="panel-heading">
            <h2>STOCKPILE RESERVES</h2>
            <span>SALE POLICY</span>
          </div>
          <div class="mineral-grid">
            <div v-for="(qty, product) in state?.products" class="inventory">
              <strong
                >{{ product }} · {{ format(qty / RESOURCE_UNIT) }} units</strong
              ><button
                @click="act('reserve', String(product), 10 * RESOURCE_UNIT)"
              >
                Keep 10</button
              ><button @click="act('reserve', String(product), 0)">
                Sell surplus
              </button>
            </div>
          </div>
        </section>
      </div>
    </section>
    <section v-else-if="tab === 'Headquarters'" class="management-view">
      <div class="management-grid">
        <section class="workshop-panel is-wide">
          <div class="panel-heading">
            <h2>PERMANENT RESEARCH</h2>
            <span>{{ state?.research ?? 0 }} RESEARCH</span>
          </div>
          <div class="research-grid">
            <button
              v-for="branch in [
                'excavation',
                'logistics',
                'metallurgy',
                'prospecting',
                'reclamation',
              ]"
              :disabled="!state"
              @click="act('research', branch)"
            >
              <strong>{{ branch }}</strong>
              <p>Rank {{ state?.ranks[branch] ?? 0 }} / 10</p>
              <small
                v-for="grant in headquartersBenefits(branch)"
                :key="grant.rank"
                >Rank {{ grant.rank }}: start with
                {{ grant.upgrades.join(" + ") }}</small
              >
              <span
                >{{
                  pacing.research_base * ((state?.ranks[branch] ?? 0) + 1) ** 2
                }}
                research</span
              >
            </button>
          </div>
        </section>
        <section class="workshop-panel">
          <div class="panel-heading">
            <h2>REBUILD BLUEPRINTS</h2>
            <span>FOREGROUND ONLY</span>
          </div>
          <p class="panel-copy">
            Logistics rank 3 unlocks foreground rebuilding. Purchases stop while
            offline.
          </p>
          <div class="action-row">
            <button @click="act('blueprint', 'camp')">Camp blueprint</button>
            <button @click="act('blueprint', 'industry')">
              Industry blueprint
            </button>
            <button @click="act('blueprint', 'off')">Disable</button>
          </div>
          <p class="panel-status">
            Queue: {{ state?.build_queue.join(" → ") || "None" }}
          </p>
        </section>
        <section class="workshop-panel">
          <div class="panel-heading">
            <h2>CAMPAIGN MILESTONES</h2>
            <span>{{ state?.milestones.length ?? 0 }} COMPLETE</span>
          </div>
          <div class="mineral-grid compact-grid">
            <div
              v-for="milestone in state?.milestones"
              :key="milestone"
              class="inventory"
            >
              ✓ {{ milestone }}
            </div>
          </div>
        </section>
        <section class="workshop-panel">
          <div class="panel-heading">
            <h2>HEADQUARTERS MEGAPROJECT</h2>
            <span>{{ state?.research_invested ?? 0 }} INVESTED</span>
          </div>
          <p class="panel-copy">
            Reserve ten units each of advanced structures, precision controls,
            magnets and batteries. Invest
            {{ pacing.headquarters_research }} research across headquarters
            branches.
          </p>
          <div class="action-row">
            <button
              :disabled="!state || state.megaproject"
              @click="act('megaproject')"
            >
              {{
                state?.megaproject
                  ? "Megaproject complete ✓"
                  : "Deliver components"
              }}
            </button>
          </div>
        </section>
        <section class="workshop-panel">
          <div class="panel-heading">
            <h2>LEAVE A LEGACY</h2>
            <span>RETIRE SITE</span>
          </div>
          <p class="panel-copy">
            Retirement keeps research, discoveries and records. Local terrain,
            buildings, workers, credits and materials reset.
          </p>
          <div class="action-row">
            <button
              class="primary"
              :disabled="!ready"
              @click="retirementPreview"
            >
              {{
                ready
                  ? "Preview retirement →"
                  : "Requires steel production and 300 m depth"
              }}
            </button>
          </div>
        </section>
      </div>
    </section>
    <section v-else-if="tab === 'Settings'" class="management-view settings">
      <div class="management-grid">
        <section class="workshop-panel">
          <div class="panel-heading">
            <h2>ACCESSIBILITY & INTERFACE</h2>
            <span>LOCAL SETTINGS</span>
          </div>
          <div class="settings-list">
            <label class="settings-row"
              ><span
                ><strong>Reduced motion</strong
                ><small
                  >Limit animated transitions and settling effects.</small
                ></span
              ><input
                v-model="preferences.reducedMotion"
                type="checkbox"
                aria-label="Reduced motion"
            /></label>
            <label class="settings-row"
              ><span
                ><strong>Interface size</strong
                ><small
                  >Scale controls and text across every screen.</small
                ></span
              ><select
                v-model.number="preferences.uiScale"
                aria-label="Interface size"
              >
                <option :value="1">100%</option>
                <option :value="1.15">115%</option>
                <option :value="1.3">130%</option>
              </select></label
            >
          </div>
        </section>
        <section class="workshop-panel">
          <div class="panel-heading">
            <h2>SOUND</h2>
            <span>MIXER</span>
          </div>
          <div class="settings-list">
            <label class="settings-row"
              ><span
                ><strong>Audio</strong><small>Enable game sound.</small></span
              ><input
                v-model="preferences.audio"
                type="checkbox"
                aria-label="Audio"
            /></label>
            <label class="settings-row"
              ><span
                ><strong>Volume</strong
                ><small>Set overall output level.</small></span
              ><input
                v-model.number="preferences.volume"
                type="range"
                min="0"
                max="1"
                step="0.01"
                aria-label="Volume"
            /></label>
            <label class="settings-row"
              ><span
                ><strong>Machinery ambience</strong
                ><small>Play continuous worksite machinery loops.</small></span
              ><input
                v-model="preferences.ambience"
                type="checkbox"
                aria-label="Machinery ambience"
            /></label>
          </div>
        </section>
        <section class="workshop-panel is-wide">
          <div class="panel-heading">
            <h2>DISPLAY</h2>
            <span>VISUAL OUTPUT</span>
          </div>
          <div class="settings-list settings-columns">
            <label class="settings-row"
              ><span
                ><strong>Visual quality</strong
                ><small>Adjust sprite and particle budgets.</small></span
              ><select
                v-model="preferences.quality"
                aria-label="Visual quality"
              >
                <option value="auto">Automatic</option>
                <option value="low">Low · fewer sprites and particles</option>
                <option value="high">High · desktop budget</option>
              </select></label
            >
            <label class="settings-row"
              ><span
                ><strong>Number display</strong
                ><small>Choose how large quantities are written.</small></span
              ><select
                v-model="preferences.numbers"
                aria-label="Number display"
              >
                <option value="compact">Compact · 12K</option>
                <option value="full">Full · 12,000</option>
                <option value="scientific">Scientific · 1.2E4</option>
              </select></label
            >
          </div>
          <p class="panel-status">
            Mineral names and patterns supplement colours. Settings remain on
            this device.
          </p>
        </section>
      </div>
    </section>
    <section v-else class="management-view records-view">
      <div class="management-grid records-grid">
        <section class="workshop-panel">
          <div class="panel-heading">
            <h2>RETIRED OPERATIONS</h2>
            <span>{{ state?.records.length ?? 0 }} ARCHIVED</span>
          </div>
          <p v-if="!state?.records.length" class="empty-state">
            Your first mine is still writing its story.
          </p>
          <div v-for="r in state?.records" :key="r.site" class="record">
            SITE {{ r.site }} · {{ r.depth }} m ·
            {{ format(r.excavated) }} cells · {{ r.research }} research
            <svg
              v-if="r.section?.length"
              viewBox="0 0 64 64"
              width="192"
              height="192"
              aria-label="Retired mine cross-section"
            >
              <rect width="64" height="64" fill="#806044" />
              <path :d="sectionPath(r.section)" fill="#101820" />
            </svg>
          </div>
        </section>
        <section class="workshop-panel">
          <div class="panel-heading">
            <h2>SAVE MANAGEMENT</h2>
            <span>LOCAL CAMPAIGN</span>
          </div>
          <p v-if="state?.requires_reset" class="panel-copy">
            This older campaign uses incompatible terrain. Export it, then start
            fresh. Reset also archives it automatically.
          </p>
          <p v-else class="panel-copy">
            Export a portable checkpoint, import another campaign, or archive
            this operation before starting again.
          </p>
          <div class="action-row vertical-actions">
            <button
              :disabled="!state"
              @click="
                showReset = true;
                resetText = '';
              "
            >
              Reset campaign
            </button>
            <button :disabled="!state" @click="exportSave">Export save</button>
            <label class="import"
              >Import save<input
                type="file"
                accept=".json"
                @change="
                  (e: Event) => {
                    const f = (e.target as HTMLInputElement).files?.[0];
                    if (f) importSave(f);
                  }
                "
            /></label>
          </div>
        </section>
      </div>
    </section>
    <footer>
      <span>DEEPWORK <b> / </b> ONE PIXEL AT A TIME.</span
      ><span>PROGRESS SAVES ON THIS DEVICE</span>
    </footer>
    <div v-if="showReset" class="modal-backdrop">
      <section
        class="card modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby="reset-title"
      >
        <h2 id="reset-title">Reset campaign?</h2>
        <p>
          This resets terrain, workers, inventory, credits, research, collection
          and records. Your current save will be archived. Audio, display and
          accessibility settings stay.
        </p>
        <label
          >Type RESET to confirm<input v-model="resetText" autocomplete="off"
        /></label>
        <button
          @click="
            showReset = false;
            resetText = '';
          "
        >
          Cancel
        </button>
        <button :disabled="resetText !== 'RESET'" @click="confirmReset">
          Start fresh campaign
        </button>
      </section>
    </div>
    <div v-if="showRetire" class="modal-backdrop">
      <section class="card modal">
        <h2>Retire this operation?</h2>
        <p>
          This resets this mine’s terrain, equipment, workforce, credits and
          inventory. Your headquarters, research and discoveries remain.
        </p>
        <p>
          Research earned:
          {{ state?.retirement_award ?? 0 }}
        </p>
        <label
          >Optional challenge
          <select v-model="selectedChallenge">
            <option value="">Standard operation</option>
            <option value="hard_rock">
              Hard rock · 50% more excavation work
            </option>
            <option value="long_haul">
              Long haul · 50% longer underground routes
            </option>
          </select>
        </label>
        <p v-if="selectedChallenge">
          Produce steel and reach 300 m at the next site for 5 extra retirement
          research. Challenge lasts for that site.
        </p>
        <div class="site-options">
          <label v-for="(profile, i) in profiles"
            ><input type="radio" v-model="selectedSite" :value="i" />
            <strong>{{ profile.name }}</strong>
            <p>{{ profile.description }}</p></label
          >
        </div>
        <button @click="retirementCancel">Keep mining</button
        ><button class="primary" @click="retirementConfirm">
          Retire & start next site
        </button>
      </section>
    </div>
  </div>
</template>
