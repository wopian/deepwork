# Implementation status — shaft-first expanded mines

Current saves use **version 9 / generator 5**. Older campaigns are archived before a fresh campaign is created; accessibility, audio and display preferences are kept separately. Old campaign benchmarks do not validate this version.

## Implemented

- Supported access starts immediately. A first worker and furnace are affordable without waiting; the lift upgrade improves later capacity and depth and no longer gates basic underground work. Required starter feed remains reserved, with explicit reserve controls.
- Signed horizontal cell/chunk keys replace surface-width indexes. World-space deterministic deposits cross chunk boundaries and the compact surface district does not bound underground exploration. Cells remain 0.25 metres and 1,000 quanta, with 64,000 quanta per material unit.
- Bounded, knowledge-limited planning connects local chambers, lifts and declines to existing supported passages. Floors and support columns remain. At equipment depth limits, lateral exploration keeps earning and surveying available.
- Survey confidence advances from approximate signals to the nearest facing ore boundary and then the remaining vein silhouette. Persistent whole-deposit orders continue after the clicked cell is mined. Public order outlines contain only revealed, unmined cells; private deposit geometry never enters IPC.
- A mine-first workshop replaces the dashboard. Desktop contextual side panels and mobile bottom sheets open equipment, processing, crew, logistics, production and contracts. Surface buildings, workers and moving cargo open relevant controls. Mouse hover and touch inspect known ore.
- Anchored pinch/wheel zoom, two-axis pan, close crew follow, surface framing and fit-workings controls. Manual movement stops following. Body/control text is 16 CSS pixels, secondary labels 14, with 44-pixel touch targets.
- Processing displays authoritative per-feed input/output rates, recovery, moving cargo, route stockpiles, sorting queue, raw storage, refinery intake, refined stock and actual reserved amounts. Pinned equipment uses native material requirements. Fractional units remain visible; no fake batch countdown.
- Four timed powers removed. Active decisions concern deposits, equipment, recipe allocation and logistics.
- Five finite transport stations and four service segments preserve batch ownership, partial unloading, priority fairness and upstream backpressure. Persisted routes support signed coordinates. Separate branches currently share these service capacities.
- Versioned offline report identities, asynchronous serial lifecycle reconciliation, checkpoint-before-publication and campaign-scoped stale-command rejection. Existing 50% rate/eight-hour cap preserved. Each return can show a new report.
- Dirty terrain textures are culled to the viewport, with a hard 768-chunk renderer cache limit. Worker and moving-particle visual budgets remain independent of simulation population. Waste uses a settling surface profile and separate economic ledgers.

## Verified in this milestone

- **109 Rust core tests**, including signed route/save validation, cross-boundary geology, hidden-ore privacy, whole-vein selection, connected negative-coordinate branches, lateral depth-gate recovery and exact offline/stepped accounting. Pending vein-accuracy work prevents idle skipping; its blocked-network regression reproduces the original failure and now matches stepped revelation.
- **11 campaign-strategy/checkpoint tests** include selecting surveyed but unexposed cells without reading hidden geology. Baseline strategies now purchase survey accuracy and keep orders tied to missing equipment or headquarters feed without switching away between survey decisions.
- **12 native persistence/lifecycle tests** cover checkpoint failure, duplicate events, stale ordering and native ownership over WebView signals.
- **27 Bun tests** pass. Vue/TypeScript checking and production frontend builds run on Windows **Bun 1.4.0**, without WSL dependency installation.
- Guided opening: **30/30 seeds** sell within 17 seconds, first refined iron in 17–290 seconds (median 23), conveyor in 33–159 seconds (median 123.5). A stronger junction check confirms actual underground forks in 22–95 seconds on all 30 seeds.
- Adverse housing-first purchase order: **30/30 seeds** recover. First iron 175–498 seconds; lift purchase 891–1,552 seconds (median 1,127.5). These are deterministic strategies, not human playtests.
- Continuous steel-first runs across all three geological profiles reach steel/specialisation in 1,073–1,497 seconds. Improved lift median is 1,908 seconds; the slowest profile takes 2,365 seconds, outside the 25–35 minute target. Lift-first adverse runs remain faster.
- Isolated packaged Windows checks pass anchored two-finger pinch, two-axis pan, production, purchases, import/export, rejection of corrupt imports, reset/archive, stale commands, preferences, background/resume and reload. Child application PATH excludes development runtimes. Desktop/portrait screenshots and emulated touch controls pass. Rotation preserves the inspected world centre and pixel scale; this is asserted in the native harness, not inferred from layout dimensions.
- Android ARM64 build script produces a debug-signed test APK and verifies v2/v3 signatures and 16 KB ZIP alignment. Rebuild after the last code checkpoint before distribution.

Sparse-depth caching preserves a 1,200-second full-state replay. Geology envelope optimization preserves all 220,242 signed-coordinate samples in a compatibility fingerprint. On a 7,020-passage mine, planner lookup/sorting optimization reduces the same 1,200-second replay from 37.6 to 18.6 seconds with identical full state. Bounded distance/work-area caches reduce that replay further to 7.6 seconds, again with identical complete state. A larger replay caught a deferred-area invalidation error missed by the initial small tests; the fix and a restored-cache regression are included. These are local fixture timings, not mobile catch-up measurements.

## Strategy intervention diagnostic

Three one-hour simulations with identical upgraded equipment compared fixed depth policy against priorities based only on discovered commodity prices. Credit changes were **+58.0%, −15.0%, and −20.5%**; the vein policy reached 174–360 metres versus 2,796–3,200 metres for depth policy. These fixture runs demonstrate a production/access tradeoff, not a guaranteed active-play bonus or campaign acceptance. No timed power multiplier is involved.

## Measurements in progress

A 30-minute expanded-terrain Windows run passed at median **58 FPS**, **86.7 MiB** peak JavaScript heap, **30** peak resident terrain textures and a **571,539-byte** compact save. Supported passages grew from 791 to 2,073 with 1,000 simulated workers represented by at most 250 sprites. Save/reload and portrait touch checks passed. The original harness falsely flagged equipment changes because JSON key order differed; recursive comparison confirms unchanged values and zero gameplay commands during measurement. Its report and corrected assessment are preserved separately.

The final 30-seed scheduled campaign uses survey purchases and public whole-vein orders. An earlier strategy that never bought survey accuracy is retained only as diagnostic evidence. Each visit now writes a full state checkpoint for reproducible diagnosis. The selected-vein endurance case also passed: 58 FPS median, 76.7 MiB peak JavaScript heap, 29 resident textures, 348,524-byte save, 636 added passages and 21,520 excavated cells. Its order survived reload with zero measurement gameplay commands. A third 30-minute endurance case passes at 58 FPS median, 80.1 MiB peak JavaScript heap, 39.4 MiB peak native working set, 12.1 MiB peak native private memory, 29 resident textures and a 333,578-byte save. It advances 1,819.3 simulation seconds during 1,819.293 measured wall seconds, with no gameplay commands. These native process measurements exclude WebView/GPU processes. Portrait rotation preserves the camera centre within floating-point error. Real mouse and touch gestures both select the same surveyed whole vein; clicked inspectors remain pinned while reaching their controls, including when the original cell is excavated. The campaign baseline began at checkpoint `3967bd4`. Twelve campaigns now continue from validated completed visits on `2d58e14`; eighteen start fresh there. Content fingerprints prevent resuming after economy/geology changes. Recorded events and elapsed campaign time are preserved, and the checkpoint test verifies identical continued simulation. The run manifest identifies each executable and continuation. A second continuation at `2480471` preserves all completed visits and runs the verified bounded planner caches; content and strategy remain unchanged. Headquarters remains targeted at 28–42 days with two daily 12-minute visits; final campaign acceptance remains pending.

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
