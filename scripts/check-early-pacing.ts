import pacing from "../content/pacing.json";
import sites from "../content/sites.json";

type Event = {
  seconds: number;
  upgrade?: string;
  product?: string;
  unlock?: string;
  specialisation?: string;
};
type FirstSiteRun = {
  profile: number;
  policy: string;
  events: Event[];
};

/** Minute-scale windows use uninterrupted first-site play, not calendar visits. */
export function assessEarlyPacing(runs: FirstSiteRun[]) {
  const errors: string[] = [];
  const profiles = new Set(runs.map((run) => run.profile));
  if (
    runs.length !== sites.length ||
    profiles.size !== sites.length ||
    sites.some((_, index) => !profiles.has(index))
  )
    errors.push("Require one first-site run for every geological profile.");
  if (runs.some((run) => run.policy !== "depth"))
    errors.push("Baseline minute targets require the depth strategy.");
  const milestones = Object.fromEntries(
    Object.entries(pacing.windows)
      .filter(([, window]) => window[1]! <= 3600)
      .map(([name, window]) => {
        const times = runs.map((run) => {
          const observations = run.events
            .filter((event) => {
              if (name === "furnace") return event.product === "iron";
              if (name === "tactics") return event.unlock === "tactics";
              if (name === "specialisation")
                return event.specialisation !== undefined;
              return event.upgrade === name;
            })
            .map((event) => event.seconds);
          if (
            !observations.length ||
            observations.some((time) => !Number.isFinite(time) || time < 0)
          ) {
            errors.push(
              `${name}: missing or invalid time for profile ${run.profile}.`,
            );
            return null;
          }
          return Math.min(...observations);
        });
        const observed = times
          .filter((time): time is number => time !== null)
          .sort((a, b) => a - b);
        const median = observed.length
          ? (observed[Math.floor((observed.length - 1) / 2)]! +
              observed[Math.floor(observed.length / 2)]!) /
            2
          : null;
        const inWindow =
          median !== null && median >= window[0]! && median <= window[1]!;
        if (!inWindow) errors.push(`${name}: median outside authored window.`);
        return [
          name,
          {
            profile_seconds: times,
            median_seconds: median,
            target_seconds: window,
            in_window: inWindow,
            all_in_window: times.every(
              (time) =>
                time !== null && time >= window[0]! && time <= window[1]!,
            ),
          },
        ];
      }),
  );
  return {
    accepted: errors.length === 0,
    profiles: profiles.size,
    milestones,
    errors,
  };
}

if (import.meta.main) {
  const paths = Bun.argv.slice(2);
  if (!paths.length)
    throw new Error(
      "Usage: bun.exe scripts/check-early-pacing.ts <profile-0.json> <profile-1.json> <profile-2.json>",
    );
  const runs = await Promise.all(paths.map((path) => Bun.file(path).json()));
  const assessment = assessEarlyPacing(runs);
  console.log(JSON.stringify(assessment, null, 2));
  if (!assessment.accepted) process.exitCode = 1;
}
