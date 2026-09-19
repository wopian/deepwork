# Implementation status — shaft-first expanded mines

Current saves use **version 10 / generator 5**. Older campaigns are archived before a fresh campaign is created; accessibility, audio and display preferences are kept separately. Old campaign benchmarks do not validate this version.

## Version 10 multi-front milestone

- Up to eight independent ore faces operate at once. Each face owns crew, work progress, route, stockpile and blocker state; headquarters excavation ranks raise the cap to twelve. A full face pauses locally while other crews keep digging.
- Whole-vein orders receive at least 60% of available diggers. Remaining crews maintain access and distinct required feeds such as iron, coal and limestone. Automatic planning and transport priorities use visible construction needs without reading hidden geology.
- Basic access now sinks one deterministic supported 8 m shaft before anonymous survey signals can pull development sideways. Guaranteed iron is discovered through normal local exposure. Guided lift purchases no longer depend on seed luck.
- Transport uses a one-second loading phase, 1.2x baseline haul rate, mode floors and at least 30% in-transit headroom over a full cycle. Starter conveyors need no power; powered lifts and trains still respect supply. Station buffer upgrades remain independent.
- Fine mine cells render at one logical pixel. Headframe, vehicles, cargo and worker positions share shaft geometry; active digger sprites spread across real mining fronts. Front route history is removed from UI snapshots while authoritative save routes remain intact.
- Headquarters ranks now grant shaft-first effects: excavation/front capacity/support speed, logistics branch rate/buffers, metallurgy recovery/power demand, prospecting refinements/accuracy and reclamation recovery. Retired open-pit grants were removed.

Current measured checks:

- Guided opening: **30/30** seeds pass. First sale 7 seconds, first refined iron 8–10 seconds, improved lift 121–166 seconds and first underground fork 19–46 seconds.
- Housing-first adverse order: **30/30** seeds pass. First refined iron 19–51 seconds, conveyor 61–106 seconds and improved lift 168–285 seconds.
- Core sweep ran 117 tests: 115 passed immediately; two assertions still encoded the former two-second loading phase, were corrected, and pass focused reruns. Campaign strategy tests pass **18/18**. Frontend tests pass **30/30**, with Bun 1.4 typecheck and production build successful.
- First version-10 campaign pilot completed five seeds at 25.5–27.0 days across all three strategies, below the four-week floor. That cohort is superseded. Headquarters research now requires 800 invested points instead of 700, adding another earned research/retirement cycle without a wall-clock gate. Fresh acceptance remains pending for this retune; version-9 results below stay historical.

## Implemented

- Supported access starts immediately. A first worker and furnace are affordable without waiting; the lift upgrade improves later capacity and depth and no longer gates basic underground work. Required starter feed remains reserved, with explicit reserve controls.
- Signed horizontal cell/chunk keys replace surface-width indexes. World-space deterministic deposits cross chunk boundaries and the compact surface district does not bound underground exploration. Cells remain 0.25 metres and 1,000 quanta, with 64,000 quanta per material unit.
- Bounded, knowledge-limited planning connects local chambers, lifts and declines to existing supported passages. Floors and support columns remain. At equipment depth limits, lateral exploration keeps earning and surveying available.
- Survey confidence advances from approximate signals to the nearest facing ore boundary and then the remaining vein silhouette. Persistent whole-deposit orders continue after the clicked cell is mined. Public order outlines contain only revealed, unmined cells; private deposit geometry and descriptor identities never enter IPC.
- A mine-first workshop replaces the dashboard. Desktop contextual side panels and mobile bottom sheets open equipment, processing, crew, logistics, production and contracts. Surface buildings, workers and moving cargo open relevant controls. Mouse hover and touch inspect known ore.
- Ore inspection names the current access limit and required lift, support, pump or ventilation upgrades from Rust. The equipment shortcut opens investment controls; the warning clears when access improves.
- Anchored pinch/wheel zoom, two-axis pan, close crew follow, surface framing and fit-workings controls. Manual movement stops following. Body/control text is 16 CSS pixels, secondary labels 14, with 44-pixel touch targets.
- Processing displays authoritative per-feed input/output rates, recovery, moving cargo, route stockpiles, sorting queue, raw storage, refinery intake, refined stock and actual reserved amounts. Pinned equipment uses native material requirements. Fractional units remain visible; no fake batch countdown.
- Four timed powers removed. Active decisions concern deposits, equipment, recipe allocation and logistics.
- Five finite transport stations and four service segments preserve batch ownership, partial unloading, priority fairness and upstream backpressure. Persisted routes support signed coordinates. Separate branches currently share these service capacities.
- Versioned offline report identities, asynchronous serial lifecycle reconciliation, checkpoint-before-publication and campaign-scoped stale-command rejection. Existing 50% rate/eight-hour cap preserved. Each return can show a new report.
- Dirty terrain textures are culled to the viewport, with a hard 768-chunk renderer cache limit. Worker and moving-particle visual budgets remain independent of simulation population. Waste uses a settling surface profile and separate economic ledgers.

## Verified in this milestone

- **112 Rust core tests**, including signed route/save validation, cross-boundary geology, hidden-ore privacy, whole-vein selection, connected negative-coordinate branches, lateral depth-gate recovery and exact offline/stepped accounting. Pending vein-accuracy work prevents idle skipping; its blocked-network regression reproduces the original failure and now matches stepped revelation.
- **18 campaign-strategy/checkpoint tests** include selecting surveyed but unexposed cells without reading hidden geology. Baseline strategies now purchase survey accuracy and keep orders tied to missing equipment or headquarters feed without switching away between survey decisions.
- **15 native persistence/lifecycle/snapshot tests** cover checkpoint failure, duplicate events, stale ordering and native ownership over WebView signals.
- **30 Bun tests** pass. Vue/TypeScript checking and production frontend builds run on Windows **Bun 1.4.0**, without WSL dependency installation.
- Guided opening: **30/30 seeds** sell within 17 seconds, first refined iron in 17–290 seconds (median 23), conveyor in 33–159 seconds (median 123.5). A stronger junction check confirms actual underground forks in 22–95 seconds on all 30 seeds.
- Adverse housing-first purchase order: **30/30 seeds** recover. First iron 175–498 seconds; lift purchase 891–1,552 seconds (median 1,127.5). These are deterministic strategies, not human playtests.
- An additional deliberate-sale diagnostic spends the first two iron on a contract and releases surplus before repinning the lift. Passive bulk recovery misses the one-hour budget on four seeds; public survey, whole-vein orders and renewed depth exploration recover 29/30 within one hour. The remaining seed recovers at 5,417 seconds (90:17). This proves a recovery path, not fast recovery after selling construction feed. Original failed strategy reports are retained.
- Continuous steel-first runs across all three geological profiles reach steel/specialisation in 1,073–1,497 seconds. Improved lift median is 1,908 seconds; the slowest profile takes 2,365 seconds, outside the 25–35 minute target. Lift-first adverse runs remain faster.
- Isolated packaged Windows checks pass anchored two-finger pinch, two-axis pan, production, purchases, import/export, rejection of corrupt imports, reset/archive, stale commands, preferences, background/resume and reload. Child application PATH excludes development runtimes. Desktop/portrait screenshots and emulated touch controls pass. Latest computed settled-state readability checks pass across 25 views (1,018 text and 533 control observations), including normal-text contrast, 16-pixel controls, 14-pixel secondary text and 44-pixel targets. Rotation preserves the inspected world centre and pixel scale; this is asserted in the native harness, not inferred from layout dimensions.
- Android ARM64 build script produces a debug-signed test APK and verifies v2/v3 signatures plus 16 KB ZIP and native ELF alignment. Current test APK includes snapshot privacy, checkpoint failure/clock-rollback fixes and access-gate inspection.

Industry separation cards name eligible source ores and use readable facility/input names. Residue quantities share the processing panel's fractional display, so nonzero work never rounds to zero. Deep overview no longer hits the old 0.15 zoom floor: camera tests fit a five-kilometre mine, and a browser-only scene check fits all 4,856 Rust-generated passages of the 3.6 km fixture in both desktop and portrait views. At overview scale, connected excavated passages remain visible even beyond the fine-texture budget. A second browser scene includes the real revealed terrain: all passages still fit, resident textures remain capped at 768, and short samples show 58 FPS desktop and 30 FPS portrait. These static scene checks are not native endurance or Android device results.

Sparse-depth caching preserves a 1,200-second full-state replay. Geology envelope optimization preserves all 220,242 signed-coordinate samples in a compatibility fingerprint. On a 7,020-passage mine, planner lookup/sorting optimization reduces the same 1,200-second replay from 37.6 to 18.6 seconds with identical full state. Bounded distance/work-area caches reduce that replay further to 7.6 seconds, again with identical complete state. A larger replay caught a deferred-area invalidation error missed by the initial small tests; the fix and a restored-cache regression are included. These are local fixture timings, not mobile catch-up measurements.

A separate 3.6 km campaign save with 4,856 passages passes an eight-real-hour offline replay against four hours of exact 20 Hz simulation. Complete serialized state matches after excluding the report and save timestamp: 30,436 cells excavated and 6,098 credits earned. Offline computation took 116.5 seconds versus 115.0 seconds for stepped simulation on this loaded Windows host. Correct accounting is verified; catch-up latency remains substantial and is not a mobile performance pass. `offline_replay` reproduces this comparison from a raw campaign diagnostic state.

The native `save_roundtrip` example accepts either native exports or raw diagnostic state. The same deep fixture encodes from 26,376,488 diagnostic bytes to a 3,512,879-byte native save and restores identical full state. Encoding took 56 ms and decoding 173 ms; those CPU timings exclude disk flush and backup work.

A subsequent dispatch fast path skips reserve/price work for empty product entries while retaining those entries in saves. On the 3.6 km fixture, a profiled 1,200-second replay drops from 7.8 to 6.0 seconds; the production build measures 5.8 seconds. Complete serialized state matches the pre-optimization replay. All 110 core tests pass. The campaign continues on identical balance content; this optimization does not alter its commands or rewards.

Repeating the full capped-offline comparison on `548a160` also passes exact state parity. Catch-up takes 89.5 seconds and equivalent stepped work 90.4 seconds, versus 116.5/115.0 seconds previously. Credits and excavated cells remain 6,098 and 30,436. Deep catch-up latency remains a practical limitation despite this improvement.

Checkpoint handling now retains the highest recorded timestamp across clock rollback, so restoring the clock cannot award an already-consumed interval again. The foreground suspension fallback computes into a candidate and saves it before replacing live state; failed persistence leaves progress and reward publication untouched, then retries. Core and native regressions cover both cases. Forward-clock simulation is unchanged, so the frozen allocation-aware campaign remains applicable. The ARM64 APK was rebuilt on bb66756 and its packaged ELF, ZIP alignment and v2/v3 signatures pass verification.

## Strategy intervention diagnostic

Three one-hour simulations with identical upgraded equipment compared fixed depth policy against priorities based only on discovered commodity prices. Credit changes were **+58.0%, −15.0%, and −20.5%**; the vein policy reached 174–360 metres versus 2,796–3,200 metres for depth policy. These fixture runs demonstrate a production/access tradeoff, not a guaranteed active-play bonus or campaign acceptance. No timed power multiplier is involved.

## Archived version 9 full campaign acceptance

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

The [committed cohort and reproduction notes](docs/balance/README.md) retain every observation. The Windows Bun-only build also passes with Node removed from process PATH.

## Performance and historical balance measurements

A 30-minute expanded-terrain Windows run passed at median **58 FPS**, **86.7 MiB** peak JavaScript heap, **30** peak resident terrain textures and a **571,539-byte** compact save. Supported passages grew from 791 to 2,073 with 1,000 simulated workers represented by at most 250 sprites. Save/reload and portrait touch checks passed. The original harness falsely flagged equipment changes because JSON key order differed; recursive comparison confirms unchanged values and zero gameplay commands during measurement. Its report and corrected assessment are preserved separately.

The current campaign retune raises electrolysis research from 275 to 300 and makes benchmark retirement recovery track equipment, depth, required ingredients and funding shortages. Unrelated junk sales no longer hide a stalled construction objective. A 350-research pilot reached first aluminium at day 21.5 on its first observed path, so that heavier gate was rejected.

The preserved 275-research baseline completed five headquarters campaigns at days 29.0, 30.5, 31.0, 31.5 and 40.0 before being stopped for retuning. These are diagnostic samples, not 30-seed acceptance. Its other checkpoints include a prolonged missing-insulation stall that motivated the recovery strategy fix.

The 300-research run completed four diagnostic headquarters campaigns at days 29.0, 30.5, 31.0 and 31.5. Its exact-state-equivalent dispatch continuation reproduced the first completed report, excluding only compute time and resume metadata. Investigation then found a benchmark strategy gap: trace residues were not mapped to eligible source ores, and a pinned lift hid headquarters ingredient priorities. Those public-catalogue decisions are now corrected; existing residue stock and progress are respected. Checkpoint fingerprints include strategy source to reject mixed decision policies.

The source-aware 300 m lift run was stopped after 18 headquarters completions (29.5–55.5 days): its earliest possible full 30-seed rare-earth median was already **28.251 days**, beyond the unchanged 28-day upper target. Unfinished and unstarted seeds were retained with conservative lower bounds, not excluded. The diagnostic report preserves every bound and completed result. This run failed timing acceptance.

Current candidate gives each lift upgrade **400 m of reach**, retaining free 300 m access and independent support, groundwater and heat gates. One authored value drives gameplay, upgrade previews and benchmark decisions. The benchmark also stops treating endless unrelated deepening as construction progress: public material tiers bound its search, while a revealed deeper required source can extend it. The observed trigger was a strategy digging beyond 7 km while missing aluminium feed.

All 30 seeds (42–71) have restarted fresh on this candidate, with two daily 12-minute visits, the existing offline cap, all three specialisations and a 56-day observation limit. Headquarters remains targeted at 28–42 days; the completed allocation-aware cohort passes those overall median gates as recorded above.

The first lift-400 cohort was superseded after 12 headquarters completions (31.0–51.5 days). Review found a strategy defect: completed headquarters component lines continued consuming shared feeds, including iron needed for lifts. The strategy now disables completed component chains and restarts them when reserved stock falls; its regression passes. All 30 seeds were restarted fresh with this allocation fix, identical gameplay content and unchanged acceptance windows. Original observations and checkpoints remain preserved; the incomplete superseded run is not acceptance.

Candidate regression repeats all 30 guided openings and 30 adverse purchase orders successfully. Guided first iron remains 17–290 seconds (median 23); adverse lift purchases remain 891–1,552 seconds. All three first-site profiles pass the unchanged median gate, with the same 2,365-second slow lift outlier. The candidate deep offline replay advances 60,300 cells and earns 7,533 credits with exact full-state parity against stepped simulation; catch-up takes 98.8 seconds and stepped computation 95.1 seconds on the loaded host. These replace the older fixture timings for current build expectations, not the historical optimization comparisons. The ARM64 candidate APK is rebuilt and signature/alignment checks pass.

The selected-vein endurance case also passed: 58 FPS median, 76.7 MiB peak JavaScript heap, 29 resident textures, 348,524-byte save, 636 added passages and 21,520 excavated cells. A third shallower case passed at 58 FPS median, 80.1 MiB peak JavaScript heap, 39.4 MiB native working set and 12.1 MiB native private memory. It advanced 1,819.3 simulation seconds during 1,819.293 wall seconds with zero gameplay commands. Native memory excludes WebView/GPU processes. Mouse and touch whole-vein selection and portrait camera preservation pass in their recorded packaged builds.

A separate 3.6 km campaign-derived stress fixture exposed oversized cargo IPC: full historic routes were repeated per visible shipment. Visual snapshots now send only the occupied leg and strip persistence-only route geometry; tests preserve positions and cargo totals. The corrected deep-mine rerun reached its initial measurement but was terminated by Windows Defender, which quarantined the unsigned test executable as `Trojan:Win32/Bearfoos.A!ml`. A second copied executable was quarantined before startup; the source executable became inaccessible. Classification remains unresolved. No exclusions or protection changes were applied. This rerun is blocked, not a completed endurance pass.

## Practical limits

Physical Android background/screen-lock/process-death/WebView recreation, touch comfort and FPS still require device testing. Desktop touch emulation and successful APK packaging do not establish those results. iOS/macOS/Linux and clean-machine Windows installation are separately unverified.

The planner is a bounded heuristic, not a proof of globally optimal lifetime layout. Support rules abstract real engineering. Distant GPU chunks are evicted; persistent sparse terrain and passage history grow with exploration. Offline active production still uses exact fixed steps, with safe skips for stationary intervals.

## Reproduce (Windows shell)

```powershell
bun.exe --version
bun.exe test
bun.exe run build
cargo test --workspace --release --lib --locked
cargo test -p mine-core --release --example campaign --locked
cargo run -p mine-core --release --example opening -- 30 guided
cargo run -p mine-core --release --example opening -- 30 adverse
cargo run -p mine-core --release --example opening -- 30 released 0 7200
cargo run -p mine-core --release --example offline_replay -- campaign-state.json 28800
cargo run -p deepwork --release --example save_roundtrip -- campaign-save.json
cargo run -p mine-core --release --example campaign -- 30 56 scheduled 12 42
bun.exe scripts/check-campaign.ts campaign-report.json
bun.exe run tauri build --no-bundle
bun.exe scripts/native-check.ts test-results/native 0 release
cargo run -p mine-core --release --example workings -- 1800 42 vein target/workings-fixture.json
bun.exe scripts/workings-native-check.ts test-results/workings 1800 --locked-input
scripts\android-test.cmd C:\absolute\output\directory
```

Campaign arguments: seed count, observation days, scheduled/attentive/continuous mode, CPU workers, first seed. Copy the harness executable to an isolated directory with a `target` subdirectory before running concurrent builds; Windows locks executing binaries. The campaign writes per-seed progress and full stalled/failed saves for reproduction. Stress setup verifies the imported fixture before measuring. Its 1,000 workers and maximum equipment are test fixtures, not balance evidence.

An optional sixth campaign argument supplies a checkpoint directory. Checkpoints include complete state, events, elapsed time, idle counters and a content fingerprint; each completed visit is written by atomic rename. Continuous-play checkpoints are not resumable through this path.
