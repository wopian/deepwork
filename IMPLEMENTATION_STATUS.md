# Implementation status — natural mines milestone

Core milestone is playable on Windows. The 30-seed scheduled campaign passes authored progression windows. Full-plan acceptance still has gaps listed below.

## Implemented

- Windows-native Bun 1.4.0, Vue 3, Vite, PixiJS 8 and Tauri 2; Rust owns simulation and geology.
- Save version 6 / generator version 2. Older terrain stays exportable and requires a fresh campaign. Typed `RESET` archives before replacing the checkpoint, preserves preferences, changes campaign identity and rejects stale commands. Failed archives/imports preserve the active campaign; malformed numeric buffer quotes are rejected.
- 512-cell terrain width, 0.25 metres per cell, 64×64 two-dimensional chunks. Each fine cell holds 1,000 quanta; one material unit holds 64,000. Shared conversions and bounded lossless RLE saves preserve scale and accounting.
- Seeded world-space veins, lenses, seams, branches, host inclusions and tapered margins. Deposits cross chunks independently of generation order. Finite reserve lenses intersect reachable workings before equipment gates.
- Neutral unrevealed ground, persistent local reveal masks, excavation exposure and local prospecting. Vein targeting reads revealed information only. Rust supplies visible chunk data.
- A 48-metre benched pit, access ramp, commissioned lift, cleared tunnel portals and bounded chambers with pillars. Routes enforce floor clearance and lift access. Indexed work faces and cached routes avoid repeatedly scanning unchanged terrain.
- Five finite station buffers and four transport segments with partial unloading, bounded coalesced batches, congestion, local upgrades, fair cargo preference and selected-segment freight boosts. Saves retain contents and travel progress. Interface shows station flow, vehicle occupancy, cycle time, loading rate, power demand and blockers.
- Finite reserve-feed storage, construction reserves and exact fractional recovery. Precision controls use simplified 275 silicon : 25 copper : 1 gallium inputs, preserving recipe mass while reducing finite-copper demand.
- Persistent waste history with shifting discharge pads, sloped shoulders, lateral expansion, bounded profile resolution, particle collision and settlement after removal. Economic contents remain independent of visual cleanup.
- Dirty chunk textures, viewport cache eviction, representative crews, bounded moving visuals, camera controls, portrait layout and accessibility/audio/display preferences. Reduced motion clears airborne waste immediately.
- Authored pacing, headquarters starting equipment, recipe controls, contracts, retirement and deterministic bulk/precision/reclamation campaign strategies. Strategy waits for first aluminium output before retiring the first electrolysis site.

## Verification

- **79 Rust core tests, 9 native tests, 3 campaign-strategy tests and 12 Bun tests pass.** Coverage includes generation order, 30-seed reserve access, local visibility, navigation, mass conservation, transport ownership/fairness, selected boosts, exact offline parity, save bounds and reset/import failures.
- Vue/TypeScript checks and frontend production build pass. Bun reports **1.4.0**. Windows frozen installation, dependency reparse-point check and restricted-PATH build without Node.js pass.
- Windows NSIS packaging and isolated native WebView2 checks pass: live production, purchase/checkpoint, reload, import/export, malformed-import rejection, reset/cancellation, stale commands and preference preservation. Touch emulation exercises buffer upgrades, cargo preference and express selection through the interface.
- Native portable tests remove development runtimes from the child application's PATH. Clean-machine installation/uninstallation is separate acceptance work.

## Campaign results

Thirty seeds (42–71), ten per specialisation, two 12-minute daily visits, capped half-rate offline progress, 56-day observation horizon. **30/30 reach headquarters; every authored day/week median passes.** The 56-day horizon checks outliers; headquarters target remains 28–42 days.

| Measured milestone | Median days | Target days |
|---|---:|---:|
| First retirement | 1.00 | 1–3 |
| Power station | 3.00 | 3–7 |
| Chemical refinery | 9.00 | 7–14 |
| First aluminium — precision-tier marker | 14.25 | 14–21 |
| First permanent magnets — rare-earth marker | 25.51 | 21–28 |
| Headquarters megaproject | 38.01 | 28–42 |

| Strategy | Seeds completed | Headquarters median | Range, days |
|---|---:|---:|---:|
| Bulk extraction | 10/10 | 41.01 | 39.01–47.00 |
| Precision refining | 10/10 | 38.01 | 37.00–39.01 |
| Reclamation | 10/10 | 31.50 | 31.00–33.00 |

These are deterministic strategy measurements, not human playtest results. Milestones measure actual products where stated. The benchmark distinguishes buying electrolysis from producing aluminium, and commissions that first line before research retirement.

Continuous first-site checks use one seed across all three geological profiles. Medians: worker purchase immediately, conveyor **4:56**, first iron **11:35**, tactics **19:11**, shaft **27:24**, specialisation **48:35**. All six early medians meet their windows. Minute-scale targets and scheduled wall-clock intervals use separate comparisons.

Matched automatic tactical use on seeds 42–44 completes headquarters **1.21% sooner, 1.34% sooner and 3.17% later**, respectively. An isolated fixed-equipment foreground hour produces **5.2–6.9% more credits**. These tests do **not** establish the 15–25% active-advantage target or cover every manual production strategy.

Continuous-play completion takes **41.51, 35.77 and 115.48 hours** for those three seeds. Continuous play supplies substantially more foreground time; compare elapsed completion separately from production gain.

## Native performance and presentation

A 30-minute persistent-waste run ends at **7,012.75 m**, **1,279,357 excavated cells** and a **2,318,460-byte compact save**. Its 171 samples include:

- 38 desktop samples: median **58 FPS**, frame P50 **21 ms**, P95 **21.7 ms**.
- 124 narrow-window samples: median **30 FPS**, P50 **31.6 ms**, P95 **42.2 ms**.
- Eight samples while another app tab hid the renderer, plus one warm-up sample.

This does not establish uninterrupted desktop 60 FPS. Maximum resident terrain textures: **72 chunks**. Maximum observed moving visuals: **600**. Maximum sampled JavaScript heap: **213.6 MiB**; native process working set: **178.2 MiB**. These exclude some WebView/GPU memory; persistent world data grows with explored terrain.

Separate fixed-1440 desktop checks maintain approximately 58 FPS with 250 representative workers. Final package passes portrait, reduced-motion, native touch transport, reload and reset checks. Reloads log Tauri fallback/old-callback warnings; no uncaught page errors occurred.

Long stress preceded later controls-recipe, interface and import-validation changes. Its fixture does not enable the changed recipe. Later shared-unit edits retain identical values. Final packaged source receives a separate native smoke run; benchmark/package provenance is recorded alongside delivered artifacts.

## Remaining acceptance work

- **Independent branch transport is not implemented:** current network serves one active-front chain. Separate branches continuing around a blocked branch remain future work.
- **Active offline aggregation is incomplete:** flowing networks use exact 20-Hz ticks. Empty/stationary intervals skip safely; fully aggregated active production remains performance work.
- **15–25% active advantage remains unmet by tested tactical policies.** Cooldown rules remain unchanged. More effective manual strategy and balancing need separate validation.
- Navigation follows authored bench/lift/drive geometry, rather than a general arbitrary tunnel planner. Waste uses a bounded surface solver rather than individual economic grains.
- Desktop 60 FPS target is not fully demonstrated. Actual Android/iOS touch, FPS, suspend/resume and packaging require device evidence; portrait emulation is insufficient.
- macOS/Linux builds and clean-machine Windows installer installation/uninstallation remain unverified.

## Reproduce

Use a Windows shell; never install dependencies from WSL.

```powershell
bun.exe install --frozen-lockfile
bun.exe run check:runtime
bun.exe test
bun.exe run build
cargo test --workspace --release --lib --locked
cargo test -p mine-core --release --example campaign --locked
cargo run -p mine-core --release --example balance -- depth 0
cmd.exe /d /c "cargo run -p mine-core --release --example campaign -- 30 56 scheduled 8 > campaign-report.json"
bun.exe run check:campaign campaign-report.json
bun.exe run tauri build --bundles nsis
bun.exe run test:native test-results 1800 release
```

Campaign arguments: seed count, observation days, scheduled/attentive/continuous mode, CPU worker count, optional starting seed. Workers are bounded by available logical processors. The strict checker requires 30 distinct seeds, all completions and all comparable milestone medians; minute-scale windows use the separate first-site benchmark.

Use separate working directories containing `target/` when running comparison processes concurrently. Windows locks running executables; use isolated Cargo target directories when recompiling alongside a benchmark.
