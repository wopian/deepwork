# Progressive mine: version 12

Workers retain identities, assignments, occupied legs and fractional travel progress. Existing assignments remain while jobs need them. Reassignment finishes the occupied leg before taking a new route. Recruitment adds deterministic identities. Shared routes live in the Rust simulation; visual snapshots carry only occupied and adjacent legs.

Walking uses 1.5 m/s. Temporary shaft ladders use 0.5 m/s and extend only through cleared shaft space. Commissioned lifts use existing lift speed, upgrades and available power. Digging, worker-driven hauling, supports and dedicated surveys require arrival. Passive surveys and conveyor automation remain. Basic access diggers build supports until engineers exist.

Developed lifts finish their final section before shallow signals divert automatic planning. Excavation and supports still require crew arrival. Explicit vein orders retain priority; finished transport remains separate from planned access.

Worker throughput rises from 1.0 to 4.5. First-site conveyors cost 100 credits and steelworks cost 600 credits. Steelworks rebuilding after retirement costs 200 credits, preserving the starter window without repeating its financing delay at every research site. Hauling throughput rises from 1.2 to 3.5. Loading delays, shipment travel timing, material requirements and physical travel speeds remain unchanged.

Excavation masks define tunnel geometry. Partial sections expose cleared endpoints and support progress independently of completed transport connections. Terrain textures add deterministic rock variation, mineral patterns and tunnel edges. Worker poses distinguish walking, climbing, lifts, digging, building, surveying and waiting. Actor coordinates remain fractional; nearest-neighbour terrain textures preserve pixel edges. Manual camera movement cancels crew follow.

## Catch-up

Rust records public visual changes during the existing offline pass. Startup, background return, imported saves and detected runtime gaps use the same path. Authoritative state stays separate from playback. Rewards become usable after successful saving; commands, import and reset stay locked while computation or playback remains active.

Playback starts when frames arrive. It waits when computation has no next frame. There is no playback duration cap or speed control. **Skip timelapse** removes animation immediately and leaves computation progress visible until saving finishes. Reduced motion skips playback automatically. Backgrounding cancels animation; the next return credits only its new interval.

Replay sessions remain available for clients connecting during startup. Adjacent frames coalesce in order when the retained data budget fills. Oversized individual snapshots use temporary compressed files, removed with their session. Retained replay storage uses at most 16 MiB RAM; normal gameplay state, IPC decoding and renderer memory are separate from this budget. Replay data is absent from campaign saves.

## Compatibility and limits

Versions 10 and 11 migrate to version 12. Generator version remains 5. Existing campaign identity, inventory, construction, excavation and transport ownership remain. New worker identities initialize deterministically; version 12 saves retain travel and batch identities.

The simulation keeps every recruited worker. Snapshots show at most 250 workers; low visual quality shows at most 100. Terrain retains at most 768 viewport textures. Moving cargo and debris share the existing actor budget. Prediction stops after 200 ms or supplied legs, including unpowered lifts and blocked arrivals. Waiting workers hold their occupied leg and do not count as travelling.

## Verification

Current evidence lives in [progressive-mine.json](balance/progressive-mine.json). Superseded opening and pacing runs remain separate, including every failure and incomplete checkpoint. A partial campaign cohort is not campaign acceptance. The required gate remains 30 distinct seeds, all three strategies, 30 headquarters completions and every unchanged overall median window.

- Bun: 36 tests pass; Vue/TypeScript checking and production build pass. Browser checks pass desktop, portrait, controls and page-error checks.
- Rust: 145 simulation tests, 27 native tests and 19 campaign strategy tests pass.
- Openings: guided, adverse and released-feed strategies each pass seeds 42–71, 30/30.
- Early pacing: every overall median gate passes. Lift times are 1,751, 1,813 and 1,764 seconds; median is 1,764 seconds. Every profile also passes. Specialisation median is 1,372 seconds. The prior model has an early 1,303-second profile 1 result below the 1,500-second lower bound; it remains retained. Earlier lift observations remain included, including the 2,202-second profile 2 outlier above the 2,100-second upper bound.
- Eight-real-hour catch-up matches exact 20 Hz advancement after excluding only report and save timestamp. Both paths credit 14,400 simulated seconds, excavate 267,838 cells and earn 13,636 credits. Computation takes 42.6 seconds offline and 40.5 seconds stepped under concurrent validation. Exact state equality holds; previous loaded-host runs remain retained.
- Generator 5 retains fingerprint `35fda104e31a4a54` across 220,242 samples.
- Isolated release WebView2 acceptance passes replay, command/import/reset locking, Skip, save-before-rewards, camera inspection, reload, portrait, touch and reduced motion. Native controls stay disabled in Industry and Headquarters during playback. A 60-second stress run retains all 1,000 simulated workers while limiting snapshots to 250. FPS is 58; maximum sampled frame p95 is 21.2 ms. Heap samples span 50,672,556–87,354,040 bytes; native memory spans 56,836–100,244 KiB. This viewport shows at most 4 moving actors and no cargo during its stress interval. The prior accepted native run retains measurements with 185 moving actors and 182 cargo actors. At most 35 textures are resident in the current viewport; the hard ceiling remains 768. Purchase IPC is 47,329 bytes; final replay snapshot is 5,041,347 bytes. These measurements validate these fixtures and host, not a full actor-budget scene, Android or long endurance. An initial portrait pinch assertion fails; a quick portrait probe and full repeated acceptance pass. That failed attempt remains retained, and its cause is not confirmed. Camera failure diagnostics now record transforms and viewport bounds.

Fresh campaign acceptance remains pending. Earlier cost models were rejected when partial observations already made a passing rare-earth median impossible. Checkpoints and the earlier failed seed 45 remain retained. The current cohort uses two independent fresh batches of 15 seeds, covering 42–71 without a serial second wave. The report combiner rejects different executables, versions, observation windows or resumed campaigns and retains every run. No timing window, research requirement, travel speed or offline rule is relaxed. A later waiting-state fix changes only count and visual functions that the campaign harness never calls. A 600-second comparison against the earlier frozen binary retains exactly equal saved simulation state. The 1.6-throughput cohort is rejected when its minimum possible rare-earth median exceeds 28 days; all 30 observations and partial checkpoints remain retained. The 2.2-throughput cohort with baseline hauling is also rejected when both rare-earth and headquarters median bounds exceed their windows. Its ten completed headquarters and all remaining observations stay retained.

The 2.0-hauling cohort also misses the rare-earth median gate: its final retained minimum possible median is 2,419,201 seconds, one second above 28 days. All 30 observations remain. Diagnosis finds developed lifts diverting toward shallow signals with 13 cells still available below their deepest passage. The current model completes this final section before automatic signal planning. Seed 42 then reaches 300 m in a 3,600-second diagnostic continuation without changing travel speeds. This continuation demonstrates recovery; campaign acceptance still uses fresh seeds. The fresh boundary-fix cohort reaches all first four calendar milestones but still cannot meet rare earths: its retained minimum possible median is 2,462,401 seconds, above 28 days. All 30 observations and checkpoints remain. Current throughput is 3.0 for workers and 2.6 for hauling; the replacement cohort starts fresh across seeds 42–71. The accepted rare-earth median window remains 21–28 days.

```powershell
bun.exe test
bun.exe run build
cargo test --workspace --release --locked
cargo test -p mine-core --release --example campaign
cargo build -p mine-core --release --examples --jobs 1
cargo run -p mine-core --release --example opening -- 30 guided 42
cargo run -p mine-core --release --example opening -- 30 adverse 42
cargo run -p mine-core --release --example opening -- 30 released 42
# Run these fresh batches concurrently in separate directories with the same campaign.exe.
# Batch A: campaign.exe 15 56 scheduled 15 42
# Batch B: campaign.exe 15 56 scheduled 15 57
bun.exe scripts/combine-campaign-reports.ts target/travel-validation-v26/campaign-report.json target/travel-validation-v26a/campaign-report.json target/travel-validation-v26b/campaign-report.json
bun.exe scripts/check-campaign.ts target/travel-validation-v26/campaign-report.json
bun.exe run test:browser test-results/travel-browser-current --built
bun.exe run test:native test-results/travel-native-current 60 release target/travel-fixture.json
```

Native acceptance uses isolated portable saves and actual WebView2. It checks desktop, portrait, touch, actor/texture budgets, save/reload, catch-up command locks, Skip, camera inspection and reduced motion. Frame intervals, heap, native memory and snapshot payload sizes remain measured evidence rather than inferred from unit tests.

The 3.0-worker/2.6-hauling cohort also misses rare earths: minimum possible median is 2,419,201 seconds. All 30 observations remain. Saved-state diagnosis finds exhausted orders with zero mineable cells, including surface remnants. These orders block automatic feed search and final shaft development. Completed orders now release at section boundaries; partially surveyed bodies retain unknown mineable cells. Both seed 45 and seed 58 reach 1,500 m during diagnostic continuations. Those continuations are not acceptance. Replacement cohort starts all 30 seeds fresh, with unchanged rates and timing gates.

The completed-order cohort also misses rare earths: retained minimum possible median is 2,419,201 seconds. All 30 observations remain, with all first four calendar gates passing. Saved-state profiling finds 90,034 travelling worker-seconds among 115,200 assigned worker-seconds. Some portal attachments choose a distant cleared loop before trying a nearby reachable portal. Routing now tries local attachments first and takes continuous cleared corridors directly, retaining every head-height clearance check, existing occupied leg and transport timing. A parallel-corridor regression covers this case. In the same diagnostic continuation, support waiting falls from 1,004 to 432 seconds and travel falls to 85,914 worker-seconds. This diagnostic is not campaign acceptance. A replacement cohort starts fresh with unchanged worker and hauling rates.

The routing cohort is rejected with all 30 observations retained: minimum possible rare-earth median is 2,440,800 seconds (28.25 days). Further review finds candidate bucket changes renaming surviving fronts and replacing valid crew jobs. Surviving fronts now keep identities, progress and cargo; new identities avoid collisions. The ownership regression passes. Stable assignments expose a slower opening, with lift median 2,807 seconds. That failed model remains retained. Current worker throughput is 4.5, hauling throughput 3.5 and first steelworks cost 600 credits; rebuilding still costs 200 credits. New qualification and fresh campaign acceptance remain pending. Physical travel, resource requirements, research and all authored windows remain unchanged.

The first retuned stable-ownership model passes all 90 openings, early overall medians and exact eight-hour parity. Native 1,000-worker stress then rejects checkpointing with Invalid mining fronts. Saved-state diagnosis finds dig credit near the one-billion validation bound. Credit survives completed face changes and banks work for future faces. Work now caps at current solid face cost; replaced faces clear credit, while unchanged faces retain progress. The clearance accounting regression begins with excessive saved credit and verifies exact mass plus zero remaining credit. Both fresh partial batches retain all 30 observations before stopping; this failure is native save validation, not a calendar gate result. New qualification remains pending.

After binding dig credit to its current face, all 90 openings, every early profile and median, 145 simulation tests, 27 native tests and 19 strategy tests pass. Exact eight-hour parity holds. Full isolated WebView2 acceptance passes the 1,000-worker stress interval, save/reload, portrait touch, replay locks and reduced motion. The frozen-save continuation did not reproduce the transient front rejection because fronts refresh before its final observation; that diagnostic limit remains recorded.

The final calendar cohort was stopped with all 30 observations retained and a rare-earth lower bound of 2,419,201 seconds. The user explicitly accepts this one-second boundary as 28 days. Rejecting that boundary was incorrect; the report checker now allows exactly one second above the rare-earth upper bound. Historical rejection files remain unchanged for audit. The stopped cohort has no headquarters completions, so it does not prove the requested 30/30 headquarters gate or final rare-earth median. Repeated progress-script runs and further travel qualification reruns are stopped. All completed functional qualifications above remain valid.
