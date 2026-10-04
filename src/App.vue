<script setup lang="ts">
import {
  computed,
  onMounted,
  onBeforeUnmount,
  nextTick,
  ref,
  watch,
} from "vue";
import World from "./World.vue";
import GameIcon from "./GameIcon.vue";
import {
  navigation,
  dockFor,
  nextGoal,
  NoticeTracker,
  type PanelName,
  type DockName,
  type MineNotice,
} from "./presentation";
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
  viewState as state,
  state as committedState,
  native,
  reconciling,
  catchup,
  commandsLocked,
  skipCatchup,
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
const headquartersBenefits: Record<string, string[]> = {
  excavation: [
    "Every rank: +4% face work",
    "Rank 3: start with powered picks",
    "Rank 6: +2 simultaneous fronts",
    "Rank 10: +2 fronts and 50% faster supports",
  ],
  logistics: [
    "Every rank: +4% loading and dispatch",
    "Rank 3: start with handcarts",
    "Rank 6: +50% shared transfer storage",
    "Rank 10: start with branch conveyors",
  ],
  metallurgy: [
    "Every rank: +4% processing",
    "Rank 3: start with sorting deck",
    "Rank 6: +5 recovery points",
    "Rank 10: 20% lower industry power demand",
  ],
  prospecting: [
    "Every rank: +5% survey work",
    "Rank 3: start with survey office",
    "Rank 6: refine two signals per survey cycle",
    "Rank 10: reveal full vein after boundary survey",
  ],
  reclamation: [
    "Every rank: +5% recovery throughput",
    "Rank 3: start with recovery screens",
    "Rank 6: +5 recovery points",
    "Rank 10: double disposal and trace throughput",
  ],
};
const showReset = ref(false);
const resetText = ref("");
const saveStatus = computed(() => {
  const status = state.value?.save_status;
  if (!status?.last_success) return "Waiting for first checkpoint";
  const bytes = status.bytes;
  const size =
    bytes < 1024
      ? `${bytes} B`
      : bytes < 1024 * 1024
        ? `${(bytes / 1024).toFixed(1)} KB`
        : `${(bytes / 1024 / 1024).toFixed(2)} MB`;
  return `Saved ${new Date(status.last_success * 1000).toLocaleString()} · ${size} · Format ${status.format}`;
});
const saveError = computed(() => state.value?.save_status.error ?? "");
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
const panel = ref<PanelName | "">("");
const drawerHeading = ref<HTMLElement>();
const drawerPanel = computed(() =>
  tab.value === "Operations" ? panel.value : (tab.value as PanelName),
);
const activeDock = computed(() =>
  drawerPanel.value ? dockFor(drawerPanel.value) : null,
);
const dockNames = Object.keys(navigation) as DockName[];
const drawerTabs = computed(() =>
  activeDock.value ? navigation[activeDock.value] : [],
);
const lastPanel: Partial<Record<DockName, PanelName>> = {};
let returnFocus: HTMLElement | null = null;
function openView(name: PanelName) {
  if (!drawerPanel.value) returnFocus = document.activeElement as HTMLElement;
  const group = dockFor(name);
  lastPanel[group] = name;
  if (
    ["Minerals", "Industry", "Headquarters", "Records", "Settings"].includes(
      name,
    )
  ) {
    tab.value = name;
    panel.value = "";
  } else {
    tab.value = "Operations";
    panel.value = name;
  }
  void nextTick(() => drawerHeading.value?.focus({ preventScroll: true }));
}
function closePanel() {
  panel.value = "";
  tab.value = "Operations";
  void nextTick(
    () =>
      returnFocus?.isConnected && returnFocus.focus({ preventScroll: true }),
  );
}
function openDock(name: DockName) {
  if (activeDock.value === name) closePanel();
  else openView(lastPanel[name] ?? navigation[name][0]!);
}
const goal = computed(() => nextGoal(committedState.value));
async function openGoal() {
  const next = goal.value;
  if (!next) return;
  openView(next.panel);
  await nextTick();
  if (next.target && next.panel === "Equipment") {
    const card = document.querySelector<HTMLElement>(
      `[data-upgrade="${CSS.escape(next.target)}"]`,
    );
    card?.scrollIntoView({ block: "center" });
    card
      ?.querySelector<HTMLButtonElement>("button:enabled")
      ?.focus({ preventScroll: true });
  }
}
const noticeTracker = new NoticeTracker();
const announcement = ref<MineNotice | null>(null);
const noticeQueue: MineNotice[] = [];
let noticeCampaign = "";
let noticeTimer: ReturnType<typeof setTimeout> | undefined;
function showNotice() {
  if (noticeTimer) clearTimeout(noticeTimer);
  announcement.value = noticeQueue.shift() ?? null;
  if (announcement.value) noticeTimer = setTimeout(showNotice, 5000);
}
watch(
  [committedState, catchup, reconciling],
  ([game, replay, saving]) => {
    if (game && game.campaign_id !== noticeCampaign) {
      noticeCampaign = game.campaign_id;
      noticeQueue.length = 0;
      announcement.value = null;
      if (noticeTimer) clearTimeout(noticeTimer);
    }
    const fresh = noticeTracker.update(game, !!replay || saving);
    if (replay || saving) {
      noticeQueue.length = 0;
      announcement.value = null;
      if (noticeTimer) clearTimeout(noticeTimer);
      return;
    }
    noticeQueue.push(...fresh);
    noticeQueue.splice(6);
    if (!announcement.value && noticeQueue.length) showNotice();
  },
  { immediate: true },
);
onBeforeUnmount(() => {
  if (noticeTimer) clearTimeout(noticeTimer);
});
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
  if (
    Object.values(navigation)
      .flat()
      .includes(name as PanelName)
  )
    openView(name as PanelName);
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
const workingCrew = computed(() => state.value?.crew_state?.working ?? 0);
onMounted(start);
</script>
<template>
  <main
    class="game-shell"
    :class="{ 'reduce-motion': preferences.reducedMotion }"
    :style="{ zoom: preferences.uiScale }"
    @keydown.esc="closePanel"
  >
    <header class="game-header">
      <div class="game-brand" aria-label="Deepwork">
        <GameIcon name="logo" /><span
          >Deepwork<small
            >Site {{ String(state?.site ?? 1).padStart(2, "0") }}</small
          ></span
        >
      </div>
      <div class="game-stats">
        <span class="stat-credit"
          ><GameIcon name="credits" /><strong>{{
            format(state?.credits ?? 0)
          }}</strong
          ><span class="stat-label">credits</span></span
        >
        <span
          ><GameIcon name="depth" /><strong>{{ depth }} m</strong></span
        >
        <button
          @click="openView('Crew')"
          :title="`${workingCrew} working · ${state?.crew_state?.travelling ?? 0} travelling · ${state?.crew_state?.blocked ?? 0} blocked`"
        >
          <GameIcon name="Crew" /><strong>{{ state?.workers ?? 3 }}</strong
          ><span>crew</span>
        </button>
      </div>
      <button
        class="checkpoint-chip"
        @click="openView('Records')"
        :class="{ fault: saveError }"
      >
        <GameIcon :name="saveError ? 'warning' : 'check'" />{{
          native ? (saveError ? "Save needs attention" : "Autosave") : "Preview"
        }}
      </button>
    </header>
    <div class="game-notices">
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
      <div
        class="notice catchup-notice"
        v-if="catchup || reconciling"
        role="status"
      >
        <span
          >{{
            catchup?.saving ? "Saving your mine…" : "Catching up your mine…"
          }}
          <template v-if="catchup">
            {{ Math.floor((catchup.done / Math.max(1, catchup.total)) * 100) }}%
            · {{ format(catchup.excavated) }} cells excavated</template
          >
        </span>
        <button v-if="catchup && !catchup.skipped" @click="skipCatchup">
          Skip timelapse
        </button>
      </div>
      <div class="notice" v-if="error && native" role="status">
        {{ error }}<button @click="error = ''" aria-label="Dismiss">×</button>
      </div>
      <details
        class="offline"
        v-if="
          !catchup &&
          state?.offline?.effective &&
          state.offline.id !== dismissedOffline
        "
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
    </div>
    <section class="mine-scene" aria-label="Mine worksite">
      <World @inspect="openPanel" />
      <button
        v-if="state"
        class="production-chip"
        @click="openView('Production')"
        :title="
          (constraint?.name ?? 'Production') +
          ': ' +
          (constraint?.blocker ?? 'Working')
        "
      >
        <span class="status-light" /><strong>{{
          constraint?.name ?? "Production"
        }}</strong
        ><span>{{ constraint?.blocker ?? "Working" }}</span
        ><GameIcon name="chevron" />
      </button>
      <div
        v-if="announcement"
        class="milestone-notice"
        role="status"
        :data-notice-id="announcement.id"
      >
        <GameIcon
          :name="announcement.kind === 'mineral' ? 'Minerals' : 'check'"
        />
        <div>
          <strong>{{ announcement.title }}</strong
          ><span>{{ announcement.detail }}</span>
        </div>
        <button @click="showNotice" aria-label="Dismiss announcement">
          <GameIcon name="close" />
        </button>
      </div>
      <div
        v-if="!preferences.tutorialDismissed && !drawerPanel && !catchup"
        class="camera-hint"
      >
        <GameIcon name="camera" /><span
          ><strong>Explore your mine</strong
          ><span class="desktop-hint"
            >Drag to explore · scroll to zoom · select ore to inspect</span
          ><span class="touch-hint"
            >Drag to explore · pinch to zoom · tap ore to inspect</span
          ></span
        >
        <button
          @click="preferences.tutorialDismissed = true"
          aria-label="Dismiss camera tutorial"
        >
          <GameIcon name="close" />
        </button>
      </div>
    </section>
    <section
      v-if="goal && preferences.guidance && !catchup && !reconciling"
      class="goal-strip"
      aria-label="Next action"
    >
      <GameIcon name="guide" />
      <div class="goal-copy">
        <small>Next chapter</small><strong>{{ goal.title }}</strong
        ><span>{{ goal.detail }}</span>
      </div>
      <progress
        :value="goal.progress"
        max="1"
        :aria-label="goal.title + ' progress'"
      />
      <button @click="openGoal" :aria-label="'View goal: ' + goal.title">
        View<GameIcon name="chevron" />
      </button>
    </section>
    <section
      v-else-if="!catchup"
      class="operation-status"
      aria-label="Mine status"
    >
      <span class="status-light" /><strong>{{
        constraint?.name ?? "Mine preview"
      }}</strong
      ><span>{{
        constraint?.blocker ??
        "Explore the worksite. Playable browser version comes later."
      }}</span>
    </section>
    <nav class="game-dock" aria-label="Mine navigation">
      <button
        v-for="name in dockNames"
        :key="name"
        @click="openDock(name)"
        :aria-pressed="activeDock === name"
      >
        <GameIcon :name="name" /><span>{{ name }}</span>
      </button>
    </nav>
    <aside
      v-if="drawerPanel"
      class="game-drawer context-panel"
      :aria-label="drawerPanel + ' controls'"
    >
      <div class="drawer-heading">
        <div>
          <small>{{ activeDock }}</small>
          <h2 ref="drawerHeading" tabindex="-1">
            {{
              drawerPanel === "Industry" ? "Recipes & reserves" : drawerPanel
            }}
          </h2>
        </div>
        <button @click="closePanel" aria-label="Close panel">
          <GameIcon name="close" />
        </button>
      </div>
      <nav
        v-if="drawerTabs.length > 1"
        class="drawer-tabs"
        aria-label="Panel sections"
      >
        <button
          v-for="name in drawerTabs"
          :key="name"
          @click="openView(name)"
          :aria-pressed="drawerPanel === name"
        >
          {{ name === "Industry" ? "Recipes" : name }}
        </button>
      </nav>
      <div class="drawer-content context-body">
        <p
          v-if="goal && goal.panel === drawerPanel && preferences.guidance"
          class="drawer-goal"
        >
          <GameIcon name="guide" /><span
            ><strong>{{ goal.title }}</strong
            >{{ goal.detail }}</span
          >
        </p>
        <template v-if="tab === 'Operations'">
          <div v-if="panel === 'Equipment'">
            <div class="panel-heading">
              <h2>Crew & equipment</h2>
              <span>UPGRADES</span>
            </div>
            <p class="crew-roster">
              Capacity estimates assume steady feed and completed construction.
            </p>
            <div class="upgrade-list">
              <div
                v-for="u in orderedUpgrades"
                :key="u[0]"
                class="upgrade-row"
                :data-upgrade="u[0]"
                :class="{
                  'is-goal': preferences.guidance && goal?.target === u[0],
                }"
              >
                <button
                  class="upgrade"
                  :disabled="
                    commandsLocked || !state || !!state.purchase_blockers[u[0]]
                  "
                  @click="act('buy', u[0])"
                >
                  <div class="upgrade-icon">
                    <GameIcon
                      :name="
                        u[0] === 'worker'
                          ? 'Crew'
                          : u[0] === 'survey'
                            ? 'survey'
                            : [
                                  'furnace',
                                  'chemical',
                                  'electrolytic',
                                  'trace',
                                ].includes(u[0])
                              ? 'Industry'
                              : 'Build'
                      "
                    />
                  </div>
                  <div>
                    <strong>{{ u[1] }}</strong>
                    <p>{{ u[2] }}</p>
                    <small>LEVEL {{ state?.levels[u[0]] ?? 0 }}</small>
                  </div>
                  <b><GameIcon name="credits" />{{ format(cost(u[0])) }}</b>
                  <small
                    class="upgrade-blocker"
                    v-if="state?.upgrade_previews[u[0]]"
                    >Machine
                    {{ format(state.upgrade_previews[u[0]].machine_percent) }}%
                    · feed line ~{{
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
                    {{ format(state.upgrade_previews[u[0]].lift_depth_after!) }}
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
                  :disabled="commandsLocked || !state"
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
              <button :disabled="commandsLocked" @click="act('clear_vein')">
                Clear vein order
              </button>
            </div>
            <p class="crew-roster" v-if="state?.crew_state">
              {{ state.crew_state.assigned }} assigned ·
              {{ state.crew_state.working }} working ·
              {{ state.crew_state.travelling }} travelling ·
              {{ state.crew_state.blocked }} blocked
            </p>
            <p class="crew-roster" v-if="state">
              <span v-for="(count, role) in state.crew" :key="role"
                >{{ count }} {{ role
                }}<template v-if="state.crew_roles?.[role]">
                  ·
                  {{ state.crew_roles[role]?.travelling }} travelling<template
                    v-if="state.crew_roles[role]?.blocked"
                  >
                    ·
                    {{ state.crew_roles[role]?.blocked }}
                    unreachable</template
                  ></template
                ></span
              >
            </p>
            <div class="mining-fronts" v-if="state?.mining_fronts.length">
              <article
                v-for="front in state.mining_fronts"
                :key="front.id"
                :class="{
                  selected: front.selected,
                  blocked: !!front.blocker,
                }"
              >
                <div>
                  <strong>{{ materials[front.material]?.name }} face</strong>
                  <small v-if="front.selected">WHOLE-VEIN PRIORITY</small>
                </div>
                <span
                  >{{ front.crew }} diggers · {{ front.haulers }} haulers ·
                  {{ format(front.face[1] / CELLS_PER_METRE) }} m</span
                >
                <progress :value="front.stockpile" :max="front.capacity" />
                <span
                  >{{ format(front.stockpile / RESOURCE_UNIT) }} /
                  {{ format(front.capacity / RESOURCE_UNIT) }} units ·
                  {{ front.blocker || front.status }}</span
                >
              </article>
            </div>
            <label class="crew-roster" v-if="state"
              >Crew priority
              <select
                :disabled="commandsLocked"
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
                <small v-if="state?.site === 1 && depth < pacing.tactics_depth"
                  >{{ pacing.tactics_depth }} M UNLOCK</small
                ></span
              ><button
                v-for="[id, label] in [
                  ['bulk', 'Broad extraction'],
                  ['vein', 'Follow veins'],
                  ['depth', 'Go deeper'],
                ]"
                :class="{ selected: state?.policy === id }"
                :disabled="
                  commandsLocked ||
                  !state ||
                  (state.site === 1 && depth < pacing.tactics_depth)
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
                :disabled="commandsLocked"
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
                >Hauling and sorting favour your three mineral priorities. Every
                fifth slot serves other cargo.</span
              >
            </label>
            <div class="mining-fronts" v-if="state?.mining_fronts.length">
              <article
                v-for="front in state.mining_fronts"
                :key="front.id"
                :class="{
                  selected: front.selected,
                  blocked: !!front.blocker,
                }"
              >
                <div>
                  <strong
                    >{{ materials[front.material]?.name }} loading bay</strong
                  >
                  <small v-if="front.selected">PRIORITY VEIN</small>
                </div>
                <span>{{ front.haulers }} haul crew</span>
                <progress :value="front.stockpile" :max="front.capacity" />
                <span
                  >{{ format(front.stockpile / RESOURCE_UNIT) }} /
                  {{ format(front.capacity / RESOURCE_UNIT) }} units ·
                  {{ front.blocker || front.status }}</span
                >
              </article>
            </div>
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
                    commandsLocked ||
                    station.level >= 50 ||
                    BigInt(state.credits) < BigInt(station.quote)
                  "
                  @click="act('buffer', station.id)"
                >
                  Buffer + · {{ format(station.quote) }} credits
                </button>
                <label
                  ><input
                    :disabled="commandsLocked"
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
                    :disabled="commandsLocked"
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
                      width: Math.min(100, (s.buffer / s.capacity) * 100) + '%',
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
                  <h2>Site specialisation</h2>
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
                      commandsLocked ||
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
                  <h2>Outgoing orders</h2>
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
                      commandsLocked ||
                      (!c.complete &&
                        (state?.products[c.product] ?? 0) < c.amount)
                    "
                    @click="
                      act(c.complete ? 'new_contract' : 'contract', '', i)
                    "
                  >
                    {{ c.complete ? "New order" : "Deliver" }}
                  </button>
                </div>
                <p v-if="!state">
                  Delivery contracts appear when native game starts.
                </p>
              </section>
            </div>
          </div>
        </template>
        <section
          v-else-if="tab === 'Minerals'"
          class="management-view catalogue"
        >
          <div class="management-grid">
            <section class="workshop-panel is-wide">
              <div class="panel-heading">
                <h2>Field guide · {{ materials.length }} FEEDS</h2>
                <input
                  :disabled="commandsLocked"
                  v-model="query"
                  placeholder="Find a mineral…"
                  aria-label="Search minerals"
                />
              </div>
              <p class="panel-copy">
                Real mineral identities. Simplified game processing. Select up
                to three excavation priorities. Cargo scheduling can favour the
                same minerals.
              </p>
              <div class="mineral-grid">
                <button
                  v-for="m in filtered"
                  :key="m.id"
                  :class="{ selected: state?.priorities.includes(m.id) }"
                  :disabled="commandsLocked || !state"
                  @click="act('priority', '', m.id)"
                >
                  <span class="mineral-swatch" :style="{ background: m.color }"
                    ><GameIcon name="Minerals" /></span
                  ><strong
                    >{{ m.name
                    }}<small
                      >{{ m.family }} ·
                      {{ m.product.replaceAll("_", " ") }}</small
                    ></strong
                  ><b>{{
                    state?.discoveries.includes(m.id) ? "FOUND" : "T" + m.tier
                  }}</b>
                </button>
              </div>
            </section>
            <section class="workshop-panel is-wide">
              <div class="panel-heading">
                <h2>Refined collection</h2>
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
                  ><GameIcon
                    :name="
                      state?.collection.includes(product) ? 'check' : 'Minerals'
                    "
                  />
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
                <h2>Processing modules</h2>
                <span>AUTOMATIC INPUT RESERVES</span>
              </div>
              <p class="panel-copy">
                Toggle production recipes. Active modules retain eight units of
                each input before selling surplus. Steel and aluminium start
                enabled; pause them to route shared feed into other products.
              </p>
              <div class="mineral-grid">
                <button
                  v-for="r in recipeCards"
                  :disabled="commandsLocked || !state"
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
                      >Recover from
                      {{ r.sources.join(" or ") }} processing.</small
                    ></strong
                  >
                </button>
              </div>
            </section>
            <section class="workshop-panel">
              <div class="panel-heading">
                <h2>Recoverable process residues</h2>
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
                <h2>Next equipment purchase</h2>
                <span>MATERIAL RESERVE</span>
              </div>
              <p class="panel-copy">
                Pin an upgrade to reserve its material cost and foundation steel
                for upcoming infrastructure. Pin again to release these
                reserves.
              </p>
              <div class="mineral-grid compact-grid">
                <button
                  :disabled="commandsLocked"
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
                <h2>Reserved feed warehouse</h2>
                <span v-if="state"
                  >{{ format(cargoTotal(state.raw_stock) / RESOURCE_UNIT) }} /
                  {{ format(state.raw_stock_capacity / RESOURCE_UNIT) }}
                  UNITS</span
                >
              </div>
              <p v-if="state" class="panel-copy">
                Reserved feed waits for its processing module. Loading depot
                upgrades add space.
              </p>
              <div class="mineral-grid">
                <div
                  v-for="m in discoveredMaterials"
                  :key="m.id"
                  class="inventory"
                >
                  <strong
                    >{{ m.name }} ·
                    {{ format((state?.raw_stock[m.id] ?? 0) / RESOURCE_UNIT) }}
                    stored</strong
                  >
                  <button
                    :disabled="commandsLocked"
                    @click="act('reserve', m.product, 4 * RESOURCE_UNIT)"
                  >
                    Reserve feed + 4 product
                  </button>
                  <button
                    :disabled="commandsLocked"
                    @click="act('reserve', m.product, 0)"
                  >
                    Release manual reserve
                  </button>
                </div>
              </div>
            </section>
            <section class="workshop-panel is-wide">
              <div class="panel-heading">
                <h2>Stockpile reserves</h2>
                <span>SALE POLICY</span>
              </div>
              <div class="mineral-grid">
                <div
                  v-for="(qty, product) in state?.products"
                  class="inventory"
                >
                  <strong
                    >{{ product }} ·
                    {{ format(qty / RESOURCE_UNIT) }} units</strong
                  ><button
                    :disabled="commandsLocked"
                    @click="act('reserve', String(product), 10 * RESOURCE_UNIT)"
                  >
                    Keep 10</button
                  ><button
                    :disabled="commandsLocked"
                    @click="act('reserve', String(product), 0)"
                  >
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
                <h2>Permanent research</h2>
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
                  :disabled="commandsLocked || !state"
                  @click="act('research', branch)"
                >
                  <strong>{{ branch }}</strong>
                  <p>Rank {{ state?.ranks[branch] ?? 0 }} / 10</p>
                  <small
                    v-for="benefit in headquartersBenefits[branch]"
                    :key="benefit"
                    >{{ benefit }}</small
                  >
                  <span
                    >{{
                      pacing.research_base *
                      ((state?.ranks[branch] ?? 0) + 1) ** 2
                    }}
                    research</span
                  >
                </button>
              </div>
            </section>
            <section class="workshop-panel">
              <div class="panel-heading">
                <h2>Rebuild blueprints</h2>
                <span>FOREGROUND ONLY</span>
              </div>
              <p class="panel-copy">
                Logistics rank 3 unlocks foreground rebuilding. Purchases stop
                while offline.
              </p>
              <div class="action-row">
                <button
                  :disabled="commandsLocked"
                  @click="act('blueprint', 'camp')"
                >
                  Camp blueprint
                </button>
                <button
                  :disabled="commandsLocked"
                  @click="act('blueprint', 'industry')"
                >
                  Industry blueprint
                </button>
                <button
                  :disabled="commandsLocked"
                  @click="act('blueprint', 'off')"
                >
                  Disable
                </button>
              </div>
              <p class="panel-status">
                Queue: {{ state?.build_queue.join(" → ") || "None" }}
              </p>
            </section>
            <section class="workshop-panel">
              <div class="panel-heading">
                <h2>Campaign milestones</h2>
                <span>{{ state?.milestones.length ?? 0 }} COMPLETE</span>
              </div>
              <div class="mineral-grid compact-grid">
                <div
                  v-for="milestone in state?.milestones"
                  :key="milestone"
                  class="inventory"
                >
                  <GameIcon name="check" />{{ milestone }}
                </div>
              </div>
            </section>
            <section class="workshop-panel">
              <div class="panel-heading">
                <h2>Headquarters megaproject</h2>
                <span>{{ state?.research_invested ?? 0 }} INVESTED</span>
              </div>
              <p class="panel-copy">
                Reserve ten units each of advanced structures, precision
                controls, magnets and batteries. Invest
                {{ pacing.headquarters_research }} research across headquarters
                branches.
              </p>
              <div class="action-row">
                <button
                  :disabled="commandsLocked || !state || state.megaproject"
                  @click="act('megaproject')"
                >
                  {{
                    state?.megaproject
                      ? "Megaproject complete"
                      : "Deliver components"
                  }}
                </button>
              </div>
            </section>
            <section class="workshop-panel">
              <div class="panel-heading">
                <h2>Leave a legacy</h2>
                <span>RETIRE SITE</span>
              </div>
              <p class="panel-copy">
                Retirement keeps research, discoveries and records. Local
                terrain, buildings, workers, credits and materials reset.
              </p>
              <div class="action-row">
                <button
                  class="primary"
                  :disabled="commandsLocked || !ready"
                  @click="retirementPreview"
                >
                  {{
                    ready
                      ? "Preview retirement"
                      : "Requires steel production and 300 m depth"
                  }}
                </button>
              </div>
            </section>
          </div>
        </section>
        <section
          v-else-if="tab === 'Settings'"
          class="management-view settings"
        >
          <div class="management-grid">
            <section class="workshop-panel">
              <div class="panel-heading">
                <h2>Accessibility & interface</h2>
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
                    ><strong>Next-action guidance</strong
                    ><small
                      >Show one optional goal throughout your campaign.</small
                    ></span
                  ><input
                    v-model="preferences.guidance"
                    type="checkbox"
                    aria-label="Next-action guidance"
                /></label>
                <div class="settings-row">
                  <span
                    ><strong>Camera tutorial</strong
                    ><small>Show visual camera instructions again.</small></span
                  ><button @click="preferences.tutorialDismissed = false">
                    Show hints
                  </button>
                </div>
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
                <h2>Sound</h2>
                <span>MIXER</span>
              </div>
              <div class="settings-list">
                <label class="settings-row"
                  ><span
                    ><strong>Audio</strong
                    ><small>Enable game sound.</small></span
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
                    ><small
                      >Play continuous worksite machinery loops.</small
                    ></span
                  ><input
                    v-model="preferences.ambience"
                    type="checkbox"
                    aria-label="Machinery ambience"
                /></label>
              </div>
            </section>
            <section class="workshop-panel is-wide">
              <div class="panel-heading">
                <h2>Display</h2>
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
                    <option value="low">
                      Low · fewer sprites and particles
                    </option>
                    <option value="high">High · desktop budget</option>
                  </select></label
                >
                <label class="settings-row"
                  ><span
                    ><strong>Number display</strong
                    ><small
                      >Choose how large quantities are written.</small
                    ></span
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
                Mineral names and patterns supplement colours. Settings remain
                on this device.
              </p>
            </section>
          </div>
        </section>
        <section v-else class="management-view records-view">
          <div class="management-grid records-grid">
            <section class="workshop-panel">
              <div class="panel-heading">
                <h2>Retired operations</h2>
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
                <h2>Save management</h2>
                <span>LOCAL CAMPAIGN</span>
              </div>
              <p v-if="state?.requires_reset" class="panel-copy">
                This older campaign uses incompatible terrain. Export it, then
                start fresh. Reset also archives it automatically.
              </p>
              <p v-else class="panel-copy">
                Export a portable checkpoint, import another campaign, or
                archive this operation before starting again.
              </p>
              <p class="panel-copy save-status">{{ saveStatus }}</p>
              <p v-if="saveError" class="panel-copy save-error" role="alert">
                {{ saveError }}
              </p>
              <div class="action-row vertical-actions">
                <button
                  :disabled="commandsLocked || !state"
                  @click="
                    showReset = true;
                    resetText = '';
                  "
                >
                  Reset campaign
                </button>
                <button
                  :disabled="commandsLocked || !state"
                  @click="exportSave"
                >
                  Export save
                </button>
                <label class="import"
                  >Import save<input
                    :disabled="commandsLocked"
                    type="file"
                    accept=".deepwork,.json"
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
      </div>
    </aside>
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
          >Type RESET to confirm<input
            :disabled="commandsLocked"
            v-model="resetText"
            autocomplete="off"
        /></label>
        <button
          @click="
            showReset = false;
            resetText = '';
          "
        >
          Cancel
        </button>
        <button
          :disabled="commandsLocked || resetText !== 'RESET'"
          @click="confirmReset"
        >
          Start fresh campaign
        </button>
      </section>
    </div>
    <div v-if="showRetire" class="modal-backdrop">
      <section
        class="card modal"
        role="dialog"
        aria-modal="true"
        aria-label="Retire this operation"
      >
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
          <select :disabled="commandsLocked" v-model="selectedChallenge">
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
            ><input
              :disabled="commandsLocked"
              type="radio"
              v-model="selectedSite"
              :value="i"
            />
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
  </main>
</template>
