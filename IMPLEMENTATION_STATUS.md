# Implementation status — dynamic underground workings

Dynamic underground mining is implemented and packaged for Windows. New geometry uses save version **7** and generator version **3**. Older saves remain exportable archives; new campaigns use the new layout. Previous fixed-drive campaign measurements do not validate this version.

## Implemented in this milestone

- Knowledge-limited automatic planning replaces periodic horizontal drives. Bounded deterministic searches weigh solid excavation, passage/lift construction, travel and existing haul distance.
- Local prospecting produces approximate signal areas and confidence. Exact mineral samples and exposed faces supply vein targets; the planner cannot query hidden geology.
- Mixed lifts and declines connect to existing supported access. Declines respect a 1:4 gradient; rails use gentler sections. Cuts preserve the pit access ramp, floor webs and support pillars.
- Failed one-metre work areas are deferred until geometry changes; known ore is not permanently abandoned after a failed approach. Already-cleared approaches are commissioned, and signals remain active until exact sampling can reach them. A reproduced shallow pit-edge campaign stall now has a permanent regression test.
- Nearby sampled ore becomes bounded extraction chambers. Chambers share existing access and receive local support; they do not create a duplicate shaft for every cell.
- Depth policy prioritises deeper surveyed access. At equipment limits, crews can return to earlier surveyed reserves. Policy and mineral-priority changes invalidate stale planning without cancelling committed work.
- Curved, branching starter reserves replace regular horizontal reserve bands. Their depths vary by seed and are independent of storage-chunk boundaries.
- Immutable cargo itineraries survive loading, partial unloading, branch changes and save/load. Quantity/route-label disagreement is rejected during import. Branches still share the five-station/four-segment service chain.
- Survey/work-plan overlay, local support columns, chamber roof beams and real lift positions replace cosmetic periodic supports. Signals include non-colour confidence marks. Display snapshots omit private search state and hidden survey history.
- Append-only passage IPC, indexed floors, cached routes, coalesced chambers and compacted collinear route points bound repeated work. Exact offline skips preserve survey phase and equipment-blocked state.

## Current verification

- **93 Rust core tests and 11 native tests pass.** Coverage includes 30-seed nonperiodic development and protected ramps, directional slope parity, local chambers, earlier-reserve recovery, hidden-information boundaries, cargo ownership, save/load and exact offline equality.
- **3 campaign-strategy tests and 15 Bun tests pass.** Vue/TypeScript checks and frontend production build pass under Bun **1.4.0**. No dependencies were installed from WSL.
- Windows NSIS packaging succeeds. Isolated native checks pass live production, purchases, reload, import/export, malformed imports, archive/reset, cancellation, stale commands and preference preservation. Touch emulation exercises transport controls.
- Native underground preview passes fixture import, save/reload, portrait layout and touch survey-overlay control, with no uncaught page errors. The packaged test application runs with development runtimes removed from its PATH.

## Pacing and long-run evidence

Final campaign and stress reports are being collected. Do not interpret earlier version-6 headquarters results as current acceptance.

Three continuous first-site geological profiles currently measure median worker purchase at startup, conveyor **4:56**, first iron **11:48**, tactics **19:51**, shaft **29:42**, and specialisation **46:34**. All six medians and all three individual profile observations meet their windows. Support construction requires 24,000 work per section (15 seconds at the baseline crew rate); engineers and support upgrades increase that rate. These are deterministic strategy measurements, not human playtests.

## Remaining full-plan acceptance

- Separate branch station networks are not implemented. Persisted cargo routes remain independent, but traffic shares station capacities; one branch cannot independently bypass a blocked shared segment.
- Active offline production still uses exact fixed steps, with safe skipping for quiet/equipment-blocked intervals. Full event-boundary aggregation remains performance work.
- No new tactical rebalance was introduced. The original 15–25% attentive-play advantage remains unproven.
- Planner is bounded and heuristic; it does not prove globally optimal lifetime layout. Rock-support and clearance rules are game abstractions, not engineering simulation.
- Actual Android/iOS hardware FPS, touch-only play, packaging and suspend/resume remain unverified. Portrait/touch emulation does not establish device acceptance.
- macOS/Linux builds and clean-machine Windows installer installation/uninstallation remain unverified.

## Reproduce

Use a Windows shell. Never install dependencies from WSL.

```powershell
bun.exe --version
bun.exe test
bun.exe run build
cargo test --workspace --release --lib --locked
cargo test -p mine-core --release --example campaign --locked
cargo run -p mine-core --release --example balance -- depth 0
# Capture profiles 0, 1 and 2 separately, then assess their JSON reports:
bun.exe scripts/check-early-pacing.ts early-0.json early-1.json early-2.json
cmd.exe /d /c "cargo run -p mine-core --release --example campaign -- 30 56 scheduled 12 > campaign-report.json"
bun.exe run check:campaign campaign-report.json
bun.exe run tauri build --bundles nsis
bun.exe scripts/native-check.ts test-results/native 0 release
cargo run -p mine-core --release --example workings -- 1800 42 vein target/workings-fixture.json
bun.exe scripts/workings-native-check.ts test-results/workings 1800
```

Campaign arguments are seed count, observation days, scheduled/attentive/continuous mode, CPU worker count and optional starting seed. Use separate report directories for concurrent campaigns. Windows locks running executables; copy harness executables before rebuilding their source target.

The underground fixture accelerates equipment and starts after a valid benched pit. Its stress run imports 1,000 workers with representative sprites and maximum local equipment. Reported renderer memory excludes some WebView/GPU allocations. Persistent terrain and passage history grow with exploration; moving visuals and resident chunk textures have separate bounded budgets.

Replay an expanded diagnostic game save with `cargo run -p mine-core --release --example replay -- stalled-save.json 120 replayed-save.json`. Optional `starter PROFILE` arguments on the workings fixture use early equipment for reserve/access regression reproduction.
