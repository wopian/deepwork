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
const rate = (n: number) =>
  n > 0 && n < 0.001
    ? "<0.001"
    : n.toLocaleString(undefined, { maximumFractionDigits: 3 });
const pinned = computed(() =>
  state.value?.pinned
    ? { id: state.value.pinned, inputs: state.value.pinned_inputs ?? {} }
    : null,
);
const label = (s: string) => s.replaceAll("_", " ");
</script>
<template>
  <section
    class="processing-panel"
    v-if="state"
    aria-label="Processing workshop"
  >
    <div v-if="pinned" class="pinned-requirements">
      <strong>Saving for {{ label(pinned.id) }}</strong>
      <div v-for="(amount, product) in pinned.inputs" :key="product">
        <span
          >{{ label(product) }} · {{ quantity(state.products[product] ?? 0) }} /
          {{ quantity(amount) }}</span
        >
        <progress
          :value="Math.min(amount, state.products[product] ?? 0)"
          :max="amount"
          :aria-label="product + ' reserved for upgrade'"
        />
      </div>
      <p v-if="!Object.keys(pinned.inputs).length">
        Credit-funded investment. {{ state.quotes[pinned.id] }} credits needed.
      </p>
    </div>
    <div v-if="!state.levels.furnace" class="furnace-investment">
      <h3>Ore needs a furnace</h3>
      <p>
        Starter ore stays reserved. Build your first workshop to produce iron.
      </p>
      <button
        @click="act('buy', 'furnace')"
        :disabled="!!state.purchase_blockers.furnace"
      >
        Build furnace · {{ state.quotes.furnace }} credits
      </button>
      <p>{{ state.purchase_blockers.furnace || "Ready to build" }}</p>
    </div>
    <article
      class="processing-feed"
      v-for="feed in state.processing ?? []"
      :key="feed.id"
    >
      <div class="feed-title">
        <span
          class="mineral-swatch"
          :style="{ background: materials[feed.id]?.color }"
          >▨</span
        >
        <strong
          >{{ materials[feed.id]?.name
          }}<small>→ {{ label(feed.output) }}</small></strong
        >
      </div>
      <p class="flow-state" :class="{ flowing: feed.output_rate > 0 }">
        <i aria-hidden="true" />{{ feed.blocker }}
      </p>
      <div class="refinery-flow">
        <span>{{ rate(feed.input_rate) }}<small>ore units / s</small></span
        ><span aria-hidden="true">→</span
        ><span
          >{{ rate(feed.output_rate)
          }}<small>{{ label(feed.output) }} / s</small></span
        >
      </div>
      <p class="recovery-label">
        {{ feed.recovery_percent }}% primary recovery · remaining contents enter
        waste recovery.
      </p>
      <dl>
        <div>
          <dt>Route stockpiles</dt>
          <dd>{{ quantity(feed.buffered) }}</dd>
        </div>
        <div>
          <dt>In transit</dt>
          <dd>{{ quantity(feed.transit) }}</dd>
        </div>
        <div>
          <dt>Sorting queue</dt>
          <dd>{{ quantity(feed.queued) }}</dd>
        </div>
        <div>
          <dt>Stored raw ore</dt>
          <dd>{{ quantity(feed.stored) }}</dd>
        </div>
        <div>
          <dt>Refinery intake</dt>
          <dd>{{ quantity(feed.intake) }}</dd>
        </div>
        <div>
          <dt>Refined stock</dt>
          <dd>{{ quantity(feed.product) }}</dd>
        </div>
        <div>
          <dt>Reserved stock</dt>
          <dd>
            {{ quantity(feed.reserved) }} / {{ quantity(feed.reserve_target) }}
          </dd>
        </div>
      </dl>
      <p class="output-destination">{{ feed.destination }}</p>
    </article>
    <p v-if="!state.processing?.length">
      No feed yet. Follow active crew to inspect excavation and hauling.
    </p>
    <p class="measurement-note">
      Rates measure current simulation-second output. Refining runs
      continuously; quantities below one unit still count.
    </p>
  </section>
</template>
