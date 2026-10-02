/** Preserve generator-specific evidence without replacing travel qualification. */
import { readFile } from "node:fs/promises";
const optionalJson = async (path: string) => {
  try {
    return JSON.parse(await readFile(path, "utf8"));
  } catch {
    return null;
  }
};
const sha256 = async (path: string) =>
  new Bun.CryptoHasher("sha256").update(await readFile(path)).digest("hex");
const current = "test-results/natural-veins-final";
const sourceFiles = [
  "crates/mine-core/src/geology.rs",
  "crates/mine-core/src/geology/natural.rs",
  "crates/mine-core/src/geometry.rs",
  "crates/mine-core/src/lib.rs",
  "crates/mine-core/src/fronts.rs",
  "crates/mine-core/src/terrain.rs",
  "crates/mine-core/src/workings.rs",
  "src-tauri/src/lib.rs",
];
const log = await readFile("target/natural-veins-workspace.log", "utf8");
const nativeLog = await readFile(
  "target/natural-veins-native-balanced.log",
  "utf8",
).catch(() => "");
const nativeUnitLog = await readFile(
  "target/natural-veins-native-unit.log",
  "utf8",
).catch(() => "");
const evidence = {
  generated_at: new Date().toISOString(),
  save_version: 12,
  generator_version: 6,
  first_feature_commit: "a046dcb",
  current_directory: current,
  source_sha256: Object.fromEntries(
    await Promise.all(
      sourceFiles.map(async (path) => [path, await sha256(path)]),
    ),
  ),
  generator: await optionalJson(`${current}/evidence.json`),
  openings: await optionalJson(`${current}/openings.json`),
  initial_generator: {
    directory: "test-results/natural-veins",
    generator: await optionalJson("test-results/natural-veins/evidence.json"),
    openings: await optionalJson("test-results/natural-veins/openings.json"),
  },
  rust_results: log
    .split(/\r?\n/)
    .filter((line) => line.startsWith("test result:")),
  final_native_unit_results: nativeUnitLog
    .split(/\r?\n/)
    .filter((line) => line.startsWith("test result:")),
  reserve_yield_check: {
    seeds: [42, 71],
    site_profiles: 3,
    minimum_estimated_primary_cells: 1500,
    sample_stride: 4,
    exact_whole_body_identity_and_uniqueness: true,
  },
  native: {
    passed: nativeLog.includes('"result": "passed"'),
    executable_sha256: await sha256("target/release/deepwork.exe"),
    fixture_sha256: await sha256(`${current}/native-fixture.json`),
    replay: await optionalJson(
      "test-results/natural-veins-native-balanced/native-replay.json",
    ),
    ipc: await optionalJson(
      "test-results/natural-veins-native-balanced/native-ipc.json",
    ),
    touch: await optionalJson(
      "test-results/natural-veins-native-balanced/touch-camera.json",
    ),
  },
  native_attempts: [
    {
      directory: "test-results/natural-veins-native",
      result:
        "Eight-hour replay timeout at 89% after 300 seconds; retained screenshot and logs.",
    },
    {
      directory: "test-results/natural-veins-native-balanced",
      passed: nativeLog.includes('"result": "passed"'),
    },
  ],
  limits: [
    "Full generator 6 calendar cohort unverified.",
    "Final openings cover nine cases; initial generator 6 covers all 90.",
    "Sampler timings measure this host under concurrent validation, not renderer FPS.",
    "Native generator 6 check does not repeat the prior 60-second 1000-worker stress test.",
    "Initial reload test omitted rebuilding transient target geometry; corrected test passes without changing persistence behavior.",
  ],
};
await Bun.write(
  "docs/balance/natural-veins.json",
  JSON.stringify(evidence, null, 2) + "\n",
);
console.log(
  JSON.stringify({
    destination: "docs/balance/natural-veins.json",
    native_passed: evidence.native.passed,
    rust_results: evidence.rust_results,
  }),
);
