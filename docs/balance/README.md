# Archived version 9 shaft-first campaign evidence

This cohort predates save version 10, independent mining fronts, new starter shaft and transport retune. It remains reproducible historical evidence, but does not validate current campaign pacing. Current version-10 results belong in this document only after all 30 fresh seeds finish.

## Full campaign acceptance

The fresh 30-seed cohort **passes all six existing overall median gates**, with **30/30 headquarters completions**. Seeds 42–71 cover ten bulk, ten precision and ten reclamation strategies. Each receives two 12-minute visits per day, 50% offline rate and the eight-real-hour cap. The observation horizon remains 56 days; no seed, failure or slow observation was excluded.

All values below are simulated calendar days. Individual outliers include observations earlier as well as later than the authored window; classification uses unrounded seconds.

| Milestone | Median | Target | Individual range | Outside window |
|---|---:|---:|---:|---:|
| Retirement | 0.503 | 0.5–3 | 0.500–1.000 | 0 |
| Power | 1.505 | 1.5–7 | 1.500–2.500 | 0 |
| Chemical | 6.752 | 5–14 | 4.504–10.500 | 2 |
| Precision | 16.252 | 14–21 | 12.000–25.502 | 20 |
| Rare Earth | 27.750 | 21–28 | 22.000–40.000 | 12 |
| Headquarters | 39.000 | 28–42 | 30.006–51.500 | 10 |

Headquarters medians by strategy: **bulk 43.753 days**, **precision 39.750 days**, **reclamation 31.006 days**. Overall acceptance is not a claim that every specialisation meets every median window independently: bulk headquarters exceeds six weeks, precision's first aluminium median is 23.003 days, and reclamation reaches aluminium earlier at 12.001 days. Ten individual headquarters observations exceed 42 days, including one by a single second; the slowest finishes at 51.500 days. These differences remain disclosed rather than rounded away.

The run used gameplay `7c1c0ca`, strategy `dcca9cf`, save version 9 and generator 5. It completed in 10,192 wall seconds on the loaded Windows host with 12 CPU workers. This is deterministic strategy evidence, not a human playtest or Android performance result. Subsequent clock/checkpoint edge-case and snapshot/UI fixes leave ordinary forward-clock simulation unchanged.

## Recheck the recorded cohort

```powershell
bun.exe scripts/check-campaign.ts docs/balance/shaft-first-campaign.json
```

The compact report retains every seed, strategy, completion flag and milestone event, plus the original executable and full-report hashes. No incomplete run was dropped. The checker reads current authored windows from `content/pacing.json`; later pacing changes can therefore invalidate this recorded result.

To reproduce simulation rather than just assess the retained observations:

```powershell
cargo run -p mine-core --release --example campaign -- 30 56 scheduled 12 42
```

Use an isolated copy of the executable if compiling other Rust targets concurrently. Full raw reports, checkpoints, logs and the generated assessment are retained with the local APK delivery under `outputs/expanded-workshop/balance`. This committed compact dataset makes all median and outlier calculations independently repeatable without those larger diagnostic saves.

Earlier 300 m and pre-allocation-fix cohorts are superseded diagnostics, not combined with this result. Opening and adverse-purchase checks remain separate from day/week campaign gates; see [implementation status](../../IMPLEMENTATION_STATUS.md).
