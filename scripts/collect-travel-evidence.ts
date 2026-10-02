/** Retain every diagnostic observation, including incomplete and failed cohorts. */
import { readdir, readFile } from "node:fs/promises";
import { join } from "node:path";
import { assessCampaign } from "./check-campaign";
import { assessEarlyPacing } from "./check-early-pacing";

const current = process.argv[2] ?? "target/travel-validation-v26";
const destination = process.argv[3] ?? "docs/balance/progressive-mine.json";
const sha256 = async (path: string) =>
  new Bun.CryptoHasher("sha256").update(await readFile(path)).digest("hex");
const optionalJson = async (path: string): Promise<any> => {
  try {
    return JSON.parse(await readFile(path, "utf8"));
  } catch {
    return null;
  }
};
const diagnostics = [];
for (const directory of (await readdir("target", { withFileTypes: true }))
  .filter(
    (entry) =>
      entry.isDirectory() && entry.name.startsWith("travel-validation"),
  )
  .sort((left, right) =>
    left.name.localeCompare(right.name, undefined, { numeric: true }),
  )) {
  const path = join("target", directory.name);
  const openings: Record<string, any[]> = {};
  const parseErrors: { file: string; line: string }[] = [];
  for (const file of (await readdir(path)).filter((name) =>
    /^opening.*\.jsonl$/.test(name),
  )) {
    openings[file] = [];
    for (const line of (await readFile(join(path, file), "utf8"))
      .split(/\r?\n/)
      .filter(Boolean)) {
      try {
        const { stages, ...observation } = JSON.parse(line);
        openings[file].push({
          ...observation,
          blockers: stages?.map((stage: any) => ({
            name: stage.name,
            blocker: stage.blocker,
          })),
        });
      } catch {
        parseErrors.push({ file, line });
      }
    }
  }
  const early = (
    await Promise.all(
      [0, 1, 2].map((profile) =>
        optionalJson(join(path, `early-${profile}.json`)),
      ),
    )
  ).filter(Boolean);
  const report = await optionalJson(join(path, "campaign-report.json"));
  const progress = await optionalJson(
    join(path, "target/campaign-progress.json"),
  );
  const checkpoints = [];
  for (const file of await readdir(join(path, "target")).catch(() => [])) {
    if (!/^campaign-checkpoint-\d+\.json$/.test(file)) continue;
    const checkpoint = await optionalJson(join(path, "target", file));
    if (!checkpoint) {
      parseErrors.push({ file, line: "Unreadable checkpoint" });
      continue;
    }
    checkpoints.push({
      seed: checkpoint.seed,
      strategy: checkpoint.style,
      mode: checkpoint.mode,
      visit: checkpoint.visit,
      wall: checkpoint.wall,
      events: checkpoint.events,
      stalls: checkpoint.stalls,
      complete: !!checkpoint.game.megaproject,
      depth:
        Math.max(0, ...Object.values(checkpoint.game.heights).map(Number)) / 4,
      site: checkpoint.game.site,
      save_version: checkpoint.game.version,
    });
  }
  const executable = join(path, "campaign.exe");
  diagnostics.push({
    directory: path,
    campaign_executable_sha256: await sha256(executable).catch(() => null),
    openings,
    early,
    early_assessment: early.length === 3 ? assessEarlyPacing(early) : null,
    campaign: report,
    campaign_rejection: await optionalJson(
      join(path, "campaign-rejection.json"),
    ),
    incomplete_progress: progress,
    checkpoints,
    offline_parity: await optionalJson(join(path, "offline-parity.json")),
    recovery: await optionalJson(join(path, "recovery-45.json")),
    boundary_recovery: await optionalJson(join(path, "recovery-42.json")),
    order_diagnosis: await optionalJson(join(path, "order-diagnosis.json")),
    order_recovery: await optionalJson(join(path, "order-recovery.json")),
    route_diagnosis: await optionalJson(join(path, "route-diagnosis.json")),
    late_states: await optionalJson(join(path, "late-states.json")),
    late_crew_diagnosis: await optionalJson(
      join(path, "late-crew-diagnosis.json"),
    ),
    native_credit_reproduction: await optionalJson(
      join(path, "native-credit-reproduction.json"),
    ),
    route_recovery: await optionalJson(join(path, "route-recovery.json")),
    support_profiles: await Promise.all(
      ["basic", "deep", "deep-66"].map((label) =>
        optionalJson(join(path, `support-profile-${label}.json`)),
      ),
    ),
    visual_only_parity: await optionalJson(join(path, "visual-parity.json")),
    validation_observations: await optionalJson(
      join(path, "validation-observations.json"),
    ),
    parseErrors,
  });
}
const campaign = await optionalJson(join(current, "campaign-report.json"));
const early = await Promise.all(
  [0, 1, 2].map((profile) =>
    optionalJson(join(current, `early-${profile}.json`)),
  ),
);
const sourceFiles = [
  "content/pacing.json",
  "crates/mine-core/src/lib.rs",
  "crates/mine-core/src/pacing.rs",
  "crates/mine-core/src/crew.rs",
  "crates/mine-core/src/fronts.rs",
  "crates/mine-core/src/terrain.rs",
  "crates/mine-core/src/transport.rs",
  "crates/mine-core/src/workings.rs",
  "crates/mine-core/examples/campaign.rs",
  "src-tauri/src/lib.rs",
  "src/game.ts",
  "src/visual-timeline.ts",
  "src/World.vue",
  "src/terrain-pixels.ts",
  "scripts/native-check.ts",
  "scripts/combine-campaign-reports.ts",
];
const evidence = {
  generated_at: new Date().toISOString(),
  save_version: 12,
  generator_version: 5,
  user_boundary_acceptance: {
    milestone: "rare_earth",
    upper_tolerance_seconds: 1,
    accepted_boundary_seconds: 2419201,
    historical_rejections_superseded:
      "One-second boundary rejection only; incomplete campaigns remain incomplete.",
  },
  current_directory: current,
  source_sha256: Object.fromEntries(
    await Promise.all(
      sourceFiles.map(async (file) => [file, await sha256(file)]),
    ),
  ),
  current_campaign: campaign,
  visual_only_parity: await optionalJson(join(current, "visual-parity.json")),
  campaign_assessment: campaign ? assessCampaign(campaign.runs) : null,
  early_assessment: early.every(Boolean) ? assessEarlyPacing(early) : null,
  geology: await optionalJson(join(current, "geology.json")),
  native: {
    executable_sha256: await sha256("target/release/deepwork.exe"),
    fixture_sha256: await sha256(
      join(current, "target/travel-fixture.json"),
    ).catch(() => null),
    replay: await optionalJson(
      "test-results/travel-native-current/native-replay.json",
    ),
    stress: await optionalJson(
      "test-results/travel-native-current/native-stress.json",
    ),
    ipc: await optionalJson(
      "test-results/travel-native-current/native-ipc.json",
    ),
  },
  native_attempts: await Promise.all(
    [
      "travel-native-release",
      "travel-native-unrestricted",
      "travel-native-final",
      "travel-native-acceptance",
      "travel-native-isolated",
      "travel-native-v15",
      "travel-native-v16",
      "travel-native-v17-initial",
      "travel-native-v17-race",
      "travel-native-v17-controls",
      "travel-native-v17-final",
      "travel-native-v18-final",
      "travel-native-v19-final",
      "travel-native-v20-final",
      "travel-native-v21-final",
      "travel-native-v22-final",
      "travel-native-v23-pinch",
      "travel-native-v23-pinch-probe",
      "travel-native-v23-repeat",
      "travel-native-v25-repeat",
      "travel-native-v26",
      "travel-native-current",
    ].map(async (name) => ({
      directory: `test-results/${name}`,
      failure: await readFile(
        `test-results/${name}/native-failure.txt`,
        "utf8",
      ).catch(() => null),
      failure_details: await optionalJson(
        `test-results/${name}/native-rejection.json`,
      ),
      replay: await optionalJson(`test-results/${name}/native-replay.json`),
      ipc: await optionalJson(`test-results/${name}/native-ipc.json`),
      stress: await optionalJson(`test-results/${name}/native-stress.json`),
    })),
  ),
  diagnostics,
};
await Bun.write(destination, JSON.stringify(evidence, null, 2) + "\n");
console.log(
  JSON.stringify({
    destination,
    accepted: evidence.campaign_assessment?.accepted ?? false,
    iterations: diagnostics.length,
    checkpoints: diagnostics.reduce(
      (total, run) => total + run.checkpoints.length,
      0,
    ),
  }),
);
