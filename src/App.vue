<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import World from "./World.vue";
import upgradeRequirements from "../content/upgrades.json";
import profiles from "../content/sites.json";
import recipes from "../content/recipes.json";
import { preferences } from "./preferences";
import {
  state,
  error,
  materials,
  upgrades,
  format,
  cost,
  act,
  start,
  exportSave,
  importSave,
} from "./game";
const materialUpgrades = upgradeRequirements.filter(
  (u) => Object.keys(u.inputs).length,
);
const selectedSite = ref(0);
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
const query = ref("");
const showRetire = ref(false);
const hideOffline = ref(false);
const depth = computed(() => Math.max(...(state.value?.heights ?? [0])) * 2);
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
  <div class="shell" :style="{ zoom: preferences.uiScale }">
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
    <div class="notice" v-if="error" role="status">
      {{ error }}<button @click="error = ''" aria-label="Dismiss">×</button>
    </div>
    <section class="offline" v-if="state?.offline?.effective && !hideOffline">
      <strong>Welcome back to the mine.</strong> Your crew excavated
      {{ format(state.offline.excavated) }} cells and earned
      {{ format(state.offline.credits) }} credits while away.<button
        @click="hideOffline = true"
      >
        Continue →
      </button>
    </section>
    <div class="page-heading">
      <div class="eyebrow">SMALL CREW. DEEP AMBITIONS.</div>
      <div class="title-row">
        <h1>
          {{
            tab === "Operations"
              ? "A little deeper, every day."
              : tab === "Minerals"
                ? "Something worth finding."
                : tab === "Headquarters"
                  ? "Build beyond this mine."
                  : "Every hole tells a story."
          }}
        </h1>
        <div class="balance">
          <small>AVAILABLE CREDITS</small
          ><strong>◈ {{ format(state?.credits ?? 0) }}</strong>
        </div>
      </div>
    </div>
    <template v-if="tab === 'Operations'">
      <div class="stats">
        <div>
          <small>DEPTH REACHED</small><strong>{{ depth }} <em>m</em></strong
          ><span
            >Next frontier ·
            {{ state?.levels.shaft ? 300 * (1 + state.levels.shaft) : 100 }}
            m</span
          >
        </div>
        <div>
          <small>ON THE PAYROLL</small
          ><strong>{{ state?.workers ?? 3 }} <em>minions</em></strong
          ><span>{{ state?.housing ?? 8 }} bunks available</span>
        </div>
        <div>
          <small>MATERIAL EXCAVATED</small
          ><strong>{{ format(state?.excavated ?? 0) }} <em>cells</em></strong
          ><span>One pixel at a time</span>
        </div>
        <div>
          <small>MINERALS DISCOVERED</small
          ><strong
            >{{ state?.discoveries.length ?? 0 }}
            <em>/ {{ materials.length }}</em></strong
          ><span>Your collection is growing</span>
        </div>
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
          <World />
          <p class="crew-roster" v-if="state"><span v-for="(count, role) in state.crew" :key="role">{{ count }} {{ role }}</span></p>
          <div class="policy">
            <span>EXCAVATION STRATEGY</span
            ><button
              v-for="[id, label] in [
                ['bulk', 'Bulk excavation'],
                ['vein', 'Follow veins'],
                ['depth', 'Go deeper'],
              ]"
              :class="{ selected: state?.policy === id }"
              :disabled="!state"
              @click="act('policy', id)"
            >
              {{ label }}
            </button>
          </div>
          <div class="pipeline">
            <div
              v-for="(s, i) in state?.stages.length
                ? state.stages
                : ['Digging', 'Hauling', 'Sorting', 'Refining', 'Dispatch'].map(
                    (name) => ({
                      name,
                      rate: 0,
                      buffer: 0,
                      capacity: 20000,
                      blocker: 'Awaiting crew',
                    }),
                  )"
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
        </section>
        <aside>
          <div class="panel-heading">
            <h2>GIVE THEM AN EDGE</h2>
            <span>UPGRADES</span>
          </div>
          <div class="upgrade-list">
            <button
              class="upgrade"
              v-for="u in orderedUpgrades"
              :key="u[0]"
              :disabled="!state || Number(state.credits) < cost(u[0])"
              @click="act('buy', u[0])"
            >
              <div class="upgrade-icon">
                {{ u[0] === "worker" ? "♟" : u[0] === "furnace" ? "♨" : "▥" }}
              </div>
              <div>
                <strong>{{ u[1] }}</strong>
                <p>{{ u[2] }}</p>
                <small>LEVEL {{ state?.levels[u[0]] ?? 0 }}</small>
              </div>
              <b>◈ {{ format(cost(u[0])) }}</b>
            </button>
          </div>
        </aside>
      </div>
      <div class="bottom-grid">
        <section class="card">
          <div class="panel-heading">
            <h2>A HAND FROM ABOVE</h2>
            <span>TACTICAL BOOSTS</span>
          </div>
          <div class="abilities">
            <button
              v-for="(label, i) in [
                'Crew rally',
                'Freight priority',
                'Furnace overdrive',
                'Directed survey',
              ]"
              :disabled="!state || !!state.cooldowns[i]"
              @click="act('ability', '', i)"
            >
              <span>{{ ["⚑", "⇢", "♨", "⌖"][i] }}</span
              ><strong>{{ label }}</strong
              ><small>{{
                state?.cooldowns[i]
                  ? `${state.cooldowns[i]}s cooldown`
                  : "READY WHEN YOU ARE"
              }}</small>
            </button>
          </div>
        </section>
        <section class="card specialisations">
          <div class="panel-heading"><h2>SITE SPECIALISATION</h2><span>{{ state?.specialisation ?? 'UNLOCKS AT 100 M' }}</span></div>
          <p>Choose once per site. New sites offer a fresh choice.</p>
          <div class="abilities specialisation-options">
            <button v-for="[id, label, detail] in [
              ['bulk', 'Bulk extraction', '+30% digging; recovery −5 percentage points.'],
              ['precision', 'Precision refining', 'Recovery +10 points (95% cap); −20% digging.'],
              ['reclamation', 'Reclamation', '3× tailings and slag recovery; −15% primary refining.'],
            ]" :class="{selected: state?.specialisation === id}" :disabled="!state || depth < 100 || !!state.specialisation" @click="act('specialise',id)">
              <strong>{{ label }}</strong><small>{{ detail }}</small>
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
                >{{ format((state?.products[c.product] ?? 0) / 1000) }} /
                {{ c.amount / 1000 }} units</small
              ></span
            ><button
              :disabled="
                !c.complete && (state?.products[c.product] ?? 0) < c.amount
              "
              @click="act(c.complete ? 'new_contract' : 'contract', '', i)"
            >
              {{ c.complete ? "New order →" : "Deliver →" }}
            </button>
          </div>
          <p v-if="!state">
            Delivery contracts appear when native game starts.
          </p>
        </section>
      </div>
    </template>
    <section v-else-if="tab === 'Minerals'" class="card catalogue">
      <div class="panel-heading">
        <h2>FIELD GUIDE · {{ materials.length }} FEEDS</h2>
        <input
          v-model="query"
          placeholder="Find a mineral…"
          aria-label="Search minerals"
        />
      </div>
      <p>
        Real mineral identities. Simplified game processing. Select up to three
        excavation priorities.
      </p>
      <div class="mineral-grid">
        <button
          v-for="m in filtered"
          :key="m.id"
          :class="{ selected: state?.priorities.includes(m.id) }"
          :disabled="!state"
          @click="act('priority', '', m.id)"
        >
          <span class="mineral-swatch" :style="{ background: m.color }">▨</span
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
    <section v-else-if="tab === 'Industry'" class="card">
      <div class="panel-heading">
        <h2>PROCESSING MODULES</h2>
        <span>AUTOMATIC INPUT RESERVES</span>
      </div>
      <p>
        Enable optional recipes. Active modules retain eight units of each input
        before selling surplus. Steel and aluminium run automatically when their
        buildings exist.
      </p>
      <div class="mineral-grid">
        <button
          v-for="r in recipes"
          :disabled="!state || ['steel', 'aluminium'].includes(r.id)"
          :class="{
            selected:
              state?.enabled_recipes.includes(r.id) ||
              ['steel', 'aluminium'].includes(r.id),
          }"
          @click="act('recipe', r.id)"
        >
          <strong
            >{{ r.output.replaceAll("_", " ")
            }}<small>{{
              Object.entries(r.inputs)
                .map(([p, n]) => `${n} ${p}`)
                .join(" + ")
            }}</small
            ><small>{{ r.building }}</small></strong
          >
        </button>
      </div>
      <h2>Recoverable process residues</h2>
      <div class="mineral-grid">
        <div v-for="(qty, name) in state?.trace_feed" class="inventory">
          {{ name }} · {{ format(qty / 1000) }} units
        </div>
      </div>
      <h2>Next equipment purchase</h2>
      <p>
        Pin an upgrade to reserve its material cost automatically. Pin again to
        release it.
      </p>
      <div class="mineral-grid">
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
            ><small>Requires {{ upgrade.requires }}</small></strong
          >
        </button>
      </div>
      <h2>Stockpile reserves</h2>
      <div class="mineral-grid">
        <div v-for="(qty, product) in state?.products" class="inventory">
          <strong>{{ product }} · {{ format(qty / 1000) }} units</strong
          ><button @click="act('reserve', String(product), 10000)">
            Keep 10</button
          ><button @click="act('reserve', String(product), 0)">
            Sell surplus
          </button>
        </div>
      </div>
    </section>
    <section v-else-if="tab === 'Headquarters'" class="card">
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
          <span>{{ 5 * ((state?.ranks[branch] ?? 0) + 1) ** 2 }} research</span>
        </button>
      </div>
      <h2>Rebuild blueprints</h2>
      <p>
        Logistics rank 3 unlocks foreground rebuilding. Purchases stop while
        offline.
      </p>
      <button @click="act('blueprint', 'camp')">Camp blueprint</button
      ><button @click="act('blueprint', 'industry')">Industry blueprint</button
      ><button @click="act('blueprint', 'off')">Disable</button>
      <p>Queue: {{ state?.build_queue.join(" → ") || "None" }}</p>
      <h2>Campaign milestones</h2>
      <div class="mineral-grid">
        <div v-for="milestone in state?.milestones" class="inventory">
          ✓ {{ milestone }}
        </div>
      </div>
      <h2>Headquarters megaproject</h2>
      <p>
        Reserve ten units each of advanced structures, precision controls,
        magnets and batteries.
      </p>
      <button
        :disabled="!state || state.megaproject"
        @click="act('megaproject')"
      >
        {{
          state?.megaproject ? "Megaproject complete ✓" : "Deliver components"
        }}
      </button>
      <h2>Leave a legacy. Start another mine.</h2>
      <p>
        Retirement keeps research, discoveries and records. Local terrain,
        buildings, workers, credits and materials reset.
      </p>
      <button class="primary" :disabled="!ready" @click="showRetire = true">
        {{
          ready
            ? "Preview retirement →"
            : "Requires steel production and 300 m depth"
        }}
      </button>
    </section>
    <section v-else-if="tab === 'Settings'" class="card settings">
      <h2>Make yourself comfortable.</h2>
      <label
        ><input v-model="preferences.reducedMotion" type="checkbox" /> Reduced
        motion</label
      ><label
        ><input v-model="preferences.audio" type="checkbox" /> Purchase
        sounds</label
      ><label
        >Volume
        <input
          v-model.number="preferences.volume"
          type="range"
          min="0"
          max="0.3"
          step="0.01" /></label
      ><label
        >Interface size
        <select v-model.number="preferences.uiScale">
          <option :value="1">100%</option>
          <option :value="1.15">115%</option>
          <option :value="1.3">130%</option>
        </select></label
      >
      <p>
        Mineral names and patterns supplement colours. Settings remain on this
        device.
      </p>
    </section>
    <section v-else class="card">
      <h2>Retired operations</h2>
      <p v-if="!state?.records.length">
        Your first mine is still writing its story.
      </p>
      <div v-for="r in state?.records" class="record">
        SITE {{ r.site }} · {{ r.depth }} m · {{ format(r.excavated) }} cells ·
        {{ r.research }} research
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
      <h2>Save management</h2>
      <button :disabled="!state" @click="exportSave">Export save</button
      ><label class="import"
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
    </section>
    <footer>
      <span>DEEPWORK <b> / </b> ONE PIXEL AT A TIME.</span
      ><span>LOCAL SAVE · RUST SIMULATION · NO CLOUD REQUIRED</span>
    </footer>
    <div v-if="showRetire" class="modal-backdrop">
      <section class="card modal">
        <h2>Retire this operation?</h2>
        <p>
          This resets this mine’s terrain, equipment, workforce, credits and
          inventory. Your headquarters, research and discoveries remain.
        </p>
        <p>
          Research earned:
          {{
            state?.retirement_award ?? 0
          }}
        </p>
        <div class="site-options">
          <label v-for="(profile, i) in profiles"
            ><input type="radio" v-model="selectedSite" :value="i" />
            <strong>{{ profile.name }}</strong>
            <p>{{ profile.description }}</p></label
          >
        </div>
        <button @click="showRetire = false">Keep mining</button
        ><button
          class="primary"
          @click="
            () => {
              act('retire', '', selectedSite);
              showRetire = false;
            }
          "
        >
          Retire & start next site
        </button>
      </section>
    </div>
  </div>
</template>
