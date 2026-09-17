import pacing from "../content/pacing.json";

type Run = {
  seed: number;
  strategy: string;
  mode: string;
  complete: boolean;
  events: Record<string, number | undefined>;
};
type Report = {
  save_version: number;
  generator_version: number;
  days: number;
  runs: Run[];
};
const median = (values: number[]) => {
  const sorted = [...values].sort((a, b) => a - b);
  return sorted.length
    ? (sorted[Math.floor((sorted.length - 1) / 2)]! +
        sorted[Math.floor(sorted.length / 2)]!) /
        2
    : null;
};
const observed = (time: number | undefined) =>
  time !== undefined && Number.isFinite(time) && time >= 0 ? time : null;

/** Paired calendar observations are not measurements of sustained throughput. */
export function compareCampaigns(baseline: Report, comparison: Report) {
  if (
    !Number.isInteger(baseline.save_version) ||
    !Number.isInteger(baseline.generator_version) ||
    baseline.save_version !== comparison.save_version ||
    baseline.generator_version !== comparison.generator_version
  )
    throw new Error("Campaign save and generator versions must match.");
  for (const report of [baseline, comparison]) {
    if (
      !Number.isFinite(report.days) ||
      report.days <= 0 ||
      !report.runs.length
    )
      throw new Error(
        "Require a positive observation horizon and campaign runs.",
      );
    const seeds = new Set(report.runs.map((run) => run.seed));
    if (
      seeds.size !== report.runs.length ||
      report.runs.some((run) => !Number.isSafeInteger(run.seed) || run.seed < 0)
    )
      throw new Error("Campaign seeds must be distinct non-negative integers.");
    if (new Set(report.runs.map((run) => run.mode)).size !== 1)
      throw new Error("Each report must use one play mode.");
  }
  const mode = comparison.runs[0]!.mode;
  if (
    baseline.runs.some((run) => run.mode !== "scheduled") ||
    !["attentive", "continuous"].includes(mode)
  )
    throw new Error(
      "Compare scheduled baseline against attentive or continuous play.",
    );
  const bySeed = new Map(baseline.runs.map((run) => [run.seed, run]));
  const pairs = comparison.runs.map((run) => {
    const base = bySeed.get(run.seed);
    if (!base || base.strategy !== run.strategy)
      throw new Error(`Seed ${run.seed} requires the same baseline strategy.`);
    return { base, run };
  });
  const milestones = Object.fromEntries(
    Object.keys(pacing.windows).map((name) => {
      const observations = pairs.map(({ base, run }) => {
        const baselineSeconds = observed(base.events[name]);
        const comparisonSeconds = observed(run.events[name]);
        return {
          seed: run.seed,
          strategy: run.strategy,
          baseline_seconds: baselineSeconds,
          comparison_seconds: comparisonSeconds,
          calendar_time_reduction_percent:
            baselineSeconds !== null &&
            baselineSeconds > 0 &&
            comparisonSeconds !== null
              ? 100 * (1 - comparisonSeconds / baselineSeconds)
              : null,
        };
      });
      const reductions = observations.flatMap((observation) =>
        observation.calendar_time_reduction_percent === null
          ? []
          : [observation.calendar_time_reduction_percent],
      );
      return [
        name,
        {
          observations,
          paired_observations: reductions.length,
          // Do not publish a survivor-only median when one run has not reached a milestone.
          median_calendar_time_reduction_percent:
            reductions.length === pairs.length ? median(reductions) : null,
        },
      ];
    }),
  );
  return {
    comparison_mode: mode,
    paired_seeds: pairs.length,
    baseline_days: baseline.days,
    comparison_days: comparison.days,
    baseline_completed: pairs.filter(({ base }) => base.complete).length,
    comparison_completed: pairs.filter(({ run }) => run.complete).length,
    unmatched_baseline_seeds: baseline.runs.length - pairs.length,
    interpretation:
      "Paired calendar milestone times; not sustained production advantage or 30-seed acceptance. Continuous play also changes active time and purchase frequency.",
    milestones,
  };
}

if (import.meta.main) {
  const [baselinePath, comparisonPath] = Bun.argv.slice(2);
  if (!baselinePath || !comparisonPath)
    throw new Error(
      "Usage: bun.exe scripts/compare-campaigns.ts <scheduled.json> <comparison.json>",
    );
  const [baseline, comparison] = await Promise.all(
    [baselinePath, comparisonPath].map((path) => Bun.file(path).json()),
  );
  console.log(JSON.stringify(compareCampaigns(baseline, comparison), null, 2));
}
