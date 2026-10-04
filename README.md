# Deepwork — Untitled Mine Game

A native mining-game prototype built with Tauri 2, Vue 3, Vite, PixiJS and a Rust simulation. Development uses **Bun 1.4.0**, including Vue template checking; Node.js is not required.

## Run on Windows

Install Rust with the MSVC toolchain, Visual Studio C++ build tools, WebView2 and Bun 1.4.0. Open a fresh Windows terminal after installing Rust.

```powershell
cd X:\GitHub\wopian\untitled-mine-game
bun.exe install --frozen-lockfile
bun.exe run tauri dev
```

Install dependencies only from a Windows shell, never from WSL. Windows executables invoked by development scripts keep Windows paths in `node_modules`.

```powershell
bun.exe test
bun.exe run build
cargo test --workspace --locked
bun.exe run tauri build --debug --no-bundle
```

The unpackaged Windows executable is `target/debug/deepwork.exe`. Browser-only Vite preview intentionally does not simulate the economy: the Rust backend is authoritative.

## Implemented

- Fine 0.25-metre terrain, natural cross-chunk deposits, local prospecting and supported shafts from the surface. Signed coordinates let branches expand beyond the compact surface district.
- Mixed lift/decline access, whole-vein orders, local support columns, retained rock floors and a survey/work-plan overlay.
- Prospecting progresses from anonymous signals to facing ore boundaries and full unmined silhouettes. Selected veins are outlined only where geology is known.
- Cargo retains its original station itinerary when crews switch branches; completed transport never teleports to the newest face.
- Worker recruitment, housing, independently upgraded transport buffers, fair cargo priorities, reserved feed and raw sales.
- 56 mineral feeds, colour-coded field guide, refined products, optional manufacturing and separation recipes.
- Explicit alumina/electrolysis path, abstract mass-conserving recipe units, steel and component production.
- Contracts, resource reserves, pausable processing recipes, power throttling, recovery, authored headquarters equipment, retirement and megaproject delivery.
- Capped half-rate idle progression uses bulk ore accounting and excavation replay. Crew travel, construction, power and capacity still constrain production. Per-return reports, compact atomic saves, archived campaign reset and stale-command protection remain.
- Warm industrial pixel art, persistent Pixi mine viewport, five-action HUD, desktop drawers, mobile bottom sheets, anchored pinch/wheel zoom, touch ore inspection and reduced motion.
- Optional campaign guidance follows pinned equipment and authored prerequisites. Goal controls reveal matching upgrades; committed discoveries receive brief notices without replay spoilers.
- Processing telemetry names ore and refined outputs, shows actual throughput and recovery, and uses the same reservation calculation as dispatch.
- Bun-only toolchain, Rust accounting tests and Windows CI.

## Current limits against the full design

Save version **12** supports generators **5 and 6**. New mines use generator 6 with mineral-specific beds, lodes, lenses, pegmatites, pipes and pockets. Connected lobes share deposit identity; compatible minerals form coherent zones. Existing generator 5 mines retain exact geology, including hidden cells. Versions 10 and 11 migrate without resetting campaigns. Worker identities, shared travel routes and cargo identities persist with inventory and excavation. Earlier unsupported campaigns remain exportable and are archived before reset. Simulation, renderer and save keys use the same signed coordinates; terrain chunks stay 64×64 cells at 0.25 metres per cell. The renderer retains at most 768 explored viewport textures.

Tunnels and supports now appear as excavation and construction advance. Underground crews travel through cleared space before producing work. Ore and workers move smoothly between snapshots. Idle returns show automatic timelapse with **Skip timelapse**; playback waits for computed frames and has no duration cap or speed controls. Reduced motion skips playback while retaining production travel delays. See [progressive mine notes](docs/progressive-mine.md).

See `docs/shaft-redesign-progress.md` for measured evidence and outstanding hardware acceptance. Campaign completion and human/device playtesting are separate checks; earlier fixed-width campaign reports do not validate this release.

## Verification and packaging

```powershell
bun.exe run test:browser
bun.exe run tauri build --debug --no-bundle
bun.exe run test:native
bun.exe run test:native test-results 1800
bun.exe run tauri build --bundles nsis
bun.exe run test:native test-results 60 release
cargo run -p mine-core --example balance -- depth 0
```

Native checks launch the actual Windows WebView2 application with isolated saves. Release checks copy the executable into a temporary directory, enable portable saves and remove development runtimes from the app's PATH. Use `deepwork.exe --portable` to keep `deepwork-data` beside a desktop executable in a writable folder. Normal launches use the platform application-data directory.

The Windows installer is produced under `target/release/bundle/nsis/`. No code-signing certificate is configured. Build success does not replace installer/device testing.

## Toolchain notes

`vue-tsc` normally intercepts CommonJS file reads to patch TypeScript. Bun bypasses that interception. `scripts/typecheck.cjs` applies Volar's transformation explicitly and runs it under Bun; template checks remain enabled. The checked tool versions and both dependency lockfiles should be committed together.

`./scripts/check-no-node.ps1` temporarily removes Node.js directories from its process PATH and builds the frontend with Bun. It does not alter system settings.

## Content

Recipes use abstract game units, not real chemical ratios. Material identities reference USGS; the proposed economics and use of mineral products are game design.

- https://www.usgs.gov/centers/national-minerals-information-center/minerals-yearbook-metals-and-minerals
- https://www.usgs.gov/programs/mineral-resources-program/science/about-2025-list-critical-minerals
- https://www.usgs.gov/programs/mineral-resources-program/minerals-and-uses-activity
- https://v2.tauri.app/start/frontend/vite/

## Save reset and old campaigns

Open **Records → Reset campaign**. Type `RESET` to archive the current campaign and start a new seed. Accessibility, audio and display settings remain. Archive/checkpoint failure aborts the reset. Earlier terrain formats stay exportable; they are not silently migrated or overwritten.

## Campaign benchmark

```powershell
cargo run -p mine-core --release --example campaign -- 30 56 scheduled 8
```

This runs 30 seeds for up to 56 days with two 12-minute visits per day and real capped offline advancement. Replace `scheduled` with `attentive` or `continuous` for comparison. Use current-version reports when assessing milestone medians; the older fixed-drive campaign results are not comparable.

The harness returns a failure status if any tested campaign misses headquarters completion. `cargo test -p mine-core --release --example campaign` checks that strategy contracts work before pumps, preserve endgame reserves, fund early processing and commission electrolysis before research retirement. Attentive strategies can prioritise surveyed deposits; they receive no hidden rate multiplier or cooldown power.

After a scheduled campaign report finishes, validate all 30 seeds and authored day/week medians:

```powershell
cmd.exe /d /c "cargo run -p mine-core --release --example campaign -- 30 56 scheduled 8 > campaign-report.json"
bun.exe run check:campaign campaign-report.json
```

This gate rejects incomplete campaigns, missing milestones, duplicate seeds and mixed timing bases. Opening checks use `cargo run --release -p mine-core --example opening -- 30 guided` and `-- 30 adverse`. Adverse runs spend starting funds on housing and release the furnace reservation before recovering through ordinary production.

The 56-day observation horizon checks late-but-completable seeds. Headquarters median target remains 28–42 days; the acceptance checker reads that unchanged target from content.

The [recorded shaft-first cohort](docs/balance/README.md) completes all 30 seeds and passes all six overall median gates. Headquarters median is 39.000 days, with individual results from 30.006 to 51.500 days. Per-specialisation timing differences and every observation remain available in that report.

## Natural underground workings

The foreman starts at the surface shaft and builds toward local survey signals and exposed ore. Survey accuracy reveals the nearest facing arc, then the full remaining vein. Click or tap revealed ore to prioritise its entire deposit; the order survives individual cuts and save/load. Unknown portions never become planner targets or client mineral tooltips.

Branches reuse supported passages and can cross zero or extend past the original surface width. Geometry is generated from deterministic world-space descriptors, independent of chunk boundaries and exploration order.

Paths account for excavation volume, walking distance, existing haul distance and lift construction. Declines stay at or below a 1:4 gradient; automatic rails use gentler sections. Cleared sections receive local support before workers use them. Rock floors and pillars constrain extraction; there is no open-pit prerequisite.

The **Survey / work plan** control shows measured signal areas and the committed local section. It does not expose hidden mineral outlines. Transport saves preserve each batch's itinerary when the working face moves.

```powershell
cargo run -p mine-core --release --example workings -- 1800 42 vein target/workings-fixture.json
bun.exe scripts/workings-native-check.ts test-results/workings 1800
```

The workings fixture contains accelerated equipment and a prepared surface entrance. It is an isolated geometry/performance scenario, not a campaign pacing result. The native harness imports it through the game UI, tests portrait touch interaction, exports and reloads the save, and records frame/cache/heap samples. Actual mobile hardware still needs separate verification.

## Android test package

Run from a Windows terminal with the Android SDK/NDK and JDK installed:

```powershell
scripts\android-test.cmd C:\path\to\output
```

This builds ARM64 assets with Windows Bun 1.4.0, signs with the local Android test key, and verifies the APK signature and ZIP alignment. Packaging checks do not establish touch performance or lifecycle behaviour on physical Android hardware.

Visual design and focused native acceptance are documented in [visual design](docs/visual-design.md). A playable Poki build still needs a browser simulation/persistence bridge and SDK integration.
