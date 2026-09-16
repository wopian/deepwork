import pacing from "../content/pacing.json";

type CampaignRun = {
  seed: number;
  strategy: string;
  mode: string;
  complete: boolean;
  events: Record<string, number>;
};
export function assessCampaign(runs: CampaignRun[]) {
  const errors: string[] = [];
  const uniqueSeeds = new Set(runs.map((run) => run.seed));
  if (runs.length < 30 || uniqueSeeds.size !== runs.length)
    errors.push("Require at least 30 distinct campaign seeds.");
  if (runs.some((run) => !Number.isSafeInteger(run.seed) || run.seed < 0))
    errors.push("Campaign seeds must be non-negative safe integers.");
  const modes = new Set(runs.map((run) => run.mode));
  if (
    modes.size !== 1 ||
    !["scheduled", "attentive"].includes(runs[0]?.mode ?? "")
  )
    errors.push("Day/week acceptance requires one two-visit campaign mode.");
  for (const strategy of ["bulk", "precision", "reclamation"])
    if (!runs.some((run) => run.strategy === strategy))
      errors.push(`Missing ${strategy} campaign strategy.`);
  const completed = runs.filter((run) => run.complete).length;
  if (completed !== runs.length)
    errors.push("Some campaigns did not complete.");
  const milestones = Object.fromEntries(
    Object.entries(pacing.windows)
      .filter(([, window]) => window[1]! > 3600)
      .map(([name, window]) => {
        const times = runs
          .map((run) => run.events[name])
          .filter(
            (time): time is number =>
              time !== undefined && Number.isFinite(time) && time >= 0,
          )
          .sort((a, b) => a - b);
        // For even samples use both central observations, not the upper one alone.
        const median = times.length
          ? (times[Math.floor((times.length - 1) / 2)]! +
              times[Math.floor(times.length / 2)]!) /
            2
          : null;
        const inWindow =
          median !== null && median >= window[0]! && median <= window[1]!;
        if (times.length !== runs.length)
          errors.push(`${name}: missing milestone observations.`);
        if (!inWindow) errors.push(`${name}: median outside authored window.`);
        return [
          name,
          {
            reached: times.length,
            median_seconds: median,
            target_seconds: window,
            in_window: inWindow,
          },
        ];
      }),
  );
  return {
    accepted: errors.length === 0,
    seeds: uniqueSeeds.size,
    completed,
    milestones,
    errors,
  };
}

if (import.meta.main) {
  const path = Bun.argv[2];
  if (!path)
    throw new Error(
      "Usage: bun.exe scripts/check-campaign.ts <campaign-report.json>",
    );
  const report = await Bun.file(path).json();
  if (!Array.isArray(report.runs))
    throw new Error("Campaign report has no runs array.");
  const assessment = assessCampaign(report.runs);
  console.log(JSON.stringify(assessment, null, 2));
  if (!assessment.accepted) process.exitCode = 1;
}
