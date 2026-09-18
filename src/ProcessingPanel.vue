<script setup lang="ts">
import { computed } from "vue";
import { state, materials, act } from "./game";
import { RESOURCE_UNIT } from "./geometry";
const quantity = (n: number) =>
  n > 0 && n < RESOURCE_UNIT / 1000
    ? "<0.001"
    : (n / RESOURCE_UNIT).toLocaleString(undefined, {
        maximumFractionDigits: 3,
      });
const stage = computed(() =>
  state.value?.stages.find((s) => s.name === "Refining"),
);
const feeds = computed(() =>
  materials.filter((m) => {
    const g = state.value;
    return (
      g &&
      [
        g.ore[m.id],
        g.hauled[m.id],
        g.raw_stock[m.id],
        g.concentrate[m.id],
        g.products[m.product],
      ].some((n) => n > 0)
    );
  }),
);
const label = (s: string) => s.replaceAll("_", " ");
</script>
<template>
  <section
    class="processing-panel"
    v-if="state"
    aria-label="Processing workshop"
  >
    <div class="panel-heading">
      <h2>Processing workshop</h2>
      <strong>{{ stage?.blocker ?? "Waiting for material" }}</strong>
    </div>
    <p v-if="!state.levels.furnace">
      Ore is arriving, but metal needs a furnace. Reserved ore stays in storage
      until equipment is ready.
    </p>
    <p v-else>
      Continuous refining ·
      {{
        (stage?.rate ?? 0).toLocaleString(undefined, {
          maximumFractionDigits: 3,
        })
      }}
      units/s through refining. Output appears as work completes.
    </p>
    <button
      v-if="!state.levels.furnace"
      @click="act('buy', 'furnace')"
      :disabled="!!state.purchase_blockers.furnace"
    >
      Build furnace · {{ state.quotes.furnace }} credits
    </button>
    <p v-if="!state.levels.furnace && state.purchase_blockers.furnace">
      {{ state.purchase_blockers.furnace }}
    </p>
    <div class="processing-feed" v-for="m in feeds" :key="m.id">
      <strong>{{ m.name }} → {{ label(m.product) }}</strong>
      <dl>
        <div>
          <dt>At work face</dt>
          <dd>{{ quantity(state.ore[m.id] ?? 0) }}</dd>
        </div>
        <div>
          <dt>Waiting for sorting</dt>
          <dd>{{ quantity(state.hauled[m.id] ?? 0) }}</dd>
        </div>
        <div>
          <dt>Stored raw feed</dt>
          <dd>{{ quantity(state.raw_stock[m.id] ?? 0) }}</dd>
        </div>
        <div>
          <dt>Refinery intake</dt>
          <dd>{{ quantity(state.concentrate[m.id] ?? 0) }}</dd>
        </div>
        <div>
          <dt>Refined stock</dt>
          <dd>{{ quantity(state.products[m.product] ?? 0) }}</dd>
        </div>
        <div>
          <dt>Product reserve target</dt>
          <dd>{{ quantity(state.reserve[m.product] ?? 0) }}</dd>
        </div>
      </dl>
    </div>
    <p v-if="!feeds.length">
      No feed yet. Follow active crew to inspect excavation and hauling.
    </p>
    <p>
      Surplus products sell automatically. Pinned upgrades and manufacturing can
      hold additional stock; held stock is not a processing delay.
    </p>
  </section>
</template>
