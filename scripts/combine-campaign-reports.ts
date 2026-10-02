/** Combine independent fresh batches without removing failures or outliers. */
import { dirname, join } from "node:path";
import { assessCampaign } from "./check-campaign";

const [destination, ...paths] = Bun.argv.slice(2);
if (!destination || paths.length < 2)
  throw new Error(
    "Usage: bun.exe scripts/combine-campaign-reports.ts DESTINATION REPORT REPORT...",
  );
const hash = async (path: string) =>
  new Bun.CryptoHasher("sha256")
    .update(new Uint8Array(await Bun.file(path).arrayBuffer()))
    .digest("hex");
const reports = await Promise.all(paths.map((path) => Bun.file(path).json()));
const first = reports[0]!;
if (
  reports.some(
    (report) =>
      report.save_version !== first.save_version ||
      report.generator_version !== first.generator_version ||
      report.days !== first.days ||
      !Array.isArray(report.runs),
  )
)
  throw new Error("Campaign batch versions or observation windows differ");
const executables = await Promise.all(
  paths.map((path) => hash(join(dirname(path), "campaign.exe"))),
);
if (new Set(executables).size !== 1)
  throw new Error("Campaign batches use different executables");
const runs = reports
  .flatMap((report) => report.runs)
  .sort((left, right) => left.seed - right.seed);
if (runs.some((run) => run.resumed_visit !== 0))
  throw new Error("Fresh cohort contains resumed campaigns");
const assessment = assessCampaign(runs);
await Bun.write(
  destination,
  JSON.stringify(
    {
      save_version: first.save_version,
      generator_version: first.generator_version,
      days: first.days,
      compute_seconds: Math.max(
        ...reports.map((report) => report.compute_seconds),
      ),
      compute_seconds_definition:
        "Slowest batch runtime; batches overlap and include native verification pauses.",
      milestone_definitions: first.milestone_definitions,
      provenance: {
        all_fresh: true,
        executable_sha256: executables[0],
        batches: await Promise.all(
          paths.map(async (path) => ({ path, sha256: await hash(path) })),
        ),
      },
      runs,
      assessment,
    },
    null,
    2,
  ) + "\n",
);
console.log(JSON.stringify(assessment));
if (!assessment.accepted) process.exitCode = 1;
