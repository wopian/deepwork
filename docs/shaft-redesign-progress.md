# Shaft redesign — 2026-09-19

The signed-coordinate expansion, whole-vein orders, contextual desktop/mobile workshop, processing telemetry, independent work fronts and fast-opening retune are implemented. Current saves are version 10 / generator 5; previous saves remain exportable archives.

See [implementation status](../IMPLEMENTATION_STATUS.md) for current checks, measured opening results, campaign/endurance status and remaining physical-device acceptance. That document supersedes the earlier fixed-width checkpoint and its incomplete-feature list.

## Final endurance and direct ore interaction

The third 30-minute Windows endurance run passed: median 58 FPS, peak JavaScript heap 80.1 MiB, native working set 39.4 MiB, native private memory 12.1 MiB, 29 resident chunks, 333,578-byte save. Measured simulation/wall time was 1,819.3/1,819.293 seconds. No gameplay commands occurred during measurement. Native memory excludes the WebView and GPU processes.

A separate packaged-app test uses actual mouse/touch drags and clicks to inspect revealed ore, move toward inspector controls and set whole-vein orders. Both modes reached the same authoritative deposit identity. Clicked inspection stays pinned; a mined anchor remains valid when its deposit still contains revealed unmined ore. Rust core checks now total 107. Physical Android acceptance remains separate.

## Bounded planner caches

Distance lookups are cached until passage geometry/scope changes. Candidate work areas are cached across failed searches, bounded to 16,384 entries. Deferred areas remain in the cache because completing a passage can clear deferrals without another geometry revision. That transition has a focused restored-state regression.

The expanded 7,020-passage fixture advances 1,200 exact simulation seconds in 7.6 seconds versus 18.6 seconds before these caches, producing identical complete serialized state. All 109 core tests pass. The 30-seed campaign continues from atomic visit checkpoints on `2480471`, retaining its prior events and content fingerprint; the manifest records each executable.

## Readability and deep cargo payloads

Computed settled-state checks cover 25 desktop/portrait views: 1,015 text and 536 control observations. They found and fixed 14-pixel policy/contract controls, default small headquarters labels, 41-pixel settings selectors and a 28-pixel mobile production shortcut. Normal text meets 4.5:1 contrast in the checked DOM views; canvas art and physical-device comfort remain separately assessed.

A campaign-derived 3.6 km endurance fixture exposed full-route cargo duplication in IPC. Its initial frame stalls reached about 10 FPS and native private memory peaked at 790 MiB; the 120-second partial run was stopped and retained as failed diagnostic evidence. Snapshots now send the occupied leg only and strip persistent batch/segment geometry without changing saved routes or material ownership. Position and snapshot-size regressions pass; totals are 110 core and 13 native tests. A new full-duration run uses the same deep fixture.

## Windows endurance blocked by Defender

The corrected 3.6 km run reached its first measurement (50 FPS, 157.7 MiB native working set, 140.0 MiB private memory), then the application closed. Windows Defender records show quarantine of the exact temporary executable/process as `Trojan:Win32/Bearfoos.A!ml` (threat ID 2147731250). A second copy was quarantined before startup, and the source executable subsequently became inaccessible. Classification is unresolved; no exclusions, restores or protection changes were performed. The initial sample is not a completed endurance result. Earlier complete endurance runs remain evidence for their recorded binaries only.

The harness now captures native stdout/stderr and exit codes. Rust tests, browser readability, Android packaging and campaign simulations remain independently verifiable.

## Final fresh campaign retune

Electrolysis now requires 300 retained research instead of 275. The 350-point diagnostic pilot delayed its first observed aluminium production to day 21.5 and was rejected. Baseline diagnostic headquarters completions were days 29.0, 30.5, 31.0, 31.5 and 40.0; the full baseline was stopped before all seeds finished.

Benchmark recovery now detects two visits without deeper access, equipment, required ingredient stock or needed funding progress. This catches sites selling abundant junk while lacking a construction feed; it uses ordinary voluntary retirement and no hidden geology. Thirteen strategy/checkpoint tests pass, including ingredient progress and income-only stagnation.

All 30 seeds restart fresh on `5ddf8cc` with unchanged pacing windows. Earlier checkpoints and reports are preserved as diagnostics, not presented as final acceptance.

## Deep offline parity

The `offline_replay` example compares a raw campaign diagnostic state under capped offline advancement and exact fixed steps. A 3.6 km save with 4,856 passages matched complete serialized state after eight real hours / four credited simulation hours, excluding only the report and checkpoint timestamp. Both paths excavated 30,436 cells and earned 6,098 credits. Local computation took 116.5 seconds offline and 115.0 seconds stepped; this proves accounting parity, not acceptable Android catch-up latency.

The native `save_roundtrip` example independently passes full-state equality for that fixture: 26,376,488 diagnostic bytes become a 3,512,879-byte compact save. Encoding/decoding takes 56/173 ms locally, excluding disk durability work.

## Source-aware campaign strategy

Late-material investigation found that benchmark priorities followed ordinary recipe ingredients but omitted eligible trace-feed sources. A pinned lift also suppressed headquarters ingredient priorities. The corrected strategy follows public trace definitions, prefers discovered eligible minerals, respects stock already in the residue ledger and counts residue production as progress. It never queries hidden geology. Fourteen strategy/checkpoint tests pass. Checkpoint fingerprints now include strategy source.

The preceding 300-research run produced four diagnostic headquarters completions at days 29, 30.5, 31 and 31.5 before this correction. All 30 seeds restart fresh for final acceptance; those earlier observations are not substituted into the new report.

## Lift-access retune after source-aware timing failure

The frozen source-aware baseline was stopped with 18 completed campaigns. Even the earliest possible 30-seed rare-earth median was 28.251 days, so the unchanged 21–28-day gate could no longer pass. All seeds remain represented in the preserved lower-bound diagnostic. Headquarters completions ranged from 29.5 to 55.5 days; this is diagnostic evidence, not final acceptance.

Lift reach now comes from validated pacing content and one shared Rust calculation. Candidate gain is 400 m per level, with initial 300 m access and support/water/heat gates unchanged. Upgrade cards show lift reach separately from actual equipment-permitted depth. Desktop and portrait synthetic-preview checks pass readability; Rust tests verify authoritative calculations. No geology, save format, material mass or recipe quantities changed.

A separate benchmark recovery fix bounds unrelated depth progress using public mineral tiers, while honouring revealed deeper required targets. This prevents a strategy from treating a 7 km detour as continued progress toward missing aluminium feed. Fresh 30-seed candidate validation remains pending.

Candidate regression: 111 core, 13 native and 17 campaign tests pass; 30 Bun tests and frontend build pass. Thirty guided and thirty adverse openings pass, as does the three-profile early median assessment. The lift-400 deep offline replay exactly matches fixed steps (60,300 cells, 7,533 credits); 98.8 seconds catch-up versus 95.1 seconds stepped, measured under campaign load. ARM64 APK rebuilt, v2/v3 signatures and 16 KB ZIP alignment verified. Fresh 30-seed run began on 7c1c0ca; full campaign acceptance remains pending.

## Snapshot metadata and deep browser terrain verification

Snapshots now strip persistence-only deposit identities, which encode private descriptor centres. Whole-vein save state remains intact; the existing native privacy regression verifies both boundaries. ARM64 APK rebuilt on a2354e3. The tracked Bun ELF guard passes the packaged native library and rejects a 4 KB-aligned negative fixture; signatures and 16 KB ZIP alignment also pass.

The deep browser check can now include real revealed terrain with --terrain. Desktop and portrait retain all 4,856 passage endpoints in frame while holding the 768-texture cap. Short frame samples were 58/30 FPS respectively, with lift-preview readability checks passing. This is static browser rendering evidence, not native endurance or Android device validation.

## Allocation-aware campaign validation

Strategy regression now verifies that completed headquarters component lines stop consuming shared feeds and restart when stock falls. This fixes a benchmark decision defect, not game rewards: the preceding lift-400 run kept making surplus magnets while iron-starved lift upgrades blocked exploration. Its 12 completed runs and all seed observations are preserved as superseded diagnostics. All 30 seeds restart fresh on gameplay 7c1c0ca and strategy dcca9cf, with unchanged windows and no mixed checkpoints. Eighteen campaign tests pass.

An additional 30-seed deliberate-sale opening diagnostic spends starter iron on a contract before repinning the lift. Public survey, vein selection and renewed exploration recover all 30; 29 finish within one hour and the remaining seed at 90:17. Passive and intermediate failed strategies remain recorded. This demonstrates recovery, not fast recovery after selling construction stock.

## Checkpoint edge cases

Clock rollback retains the last accounted timestamp across offline advancement, background events, autosaves and commands. Restoring the clock cannot re-award the consumed interval. The foreground suspension fallback now saves a cloned candidate before replacing live state; failed saves preserve the original state and retry without publishing rewards. Tests pass: 112 core, 14 native, 18 campaign. Normal forward-clock simulation is unchanged; the active campaign executable remains valid for the balance gate.

ARM64 APK rebuilt on bb66756; v2/v3 signatures, 16 KB ZIP alignment and all three packaged ELF load segments pass. SHA-256: 5dbe2eceeeb4f0ebd60ef9cf969d08436d44fc4abd728c886f8e2d157f5fe011. Physical Android checks remain pending.

## Ore access feedback

Snapshots expose the authoritative current depth limit and equipment needed to extend it. Ore inspection shows that limit, names required upgrades, and opens Equipment without rejecting a whole-vein order that still has accessible faces. Native tests cover simultaneous lift/support gates, drainage, heat and maximum lift reach. Desktop click and portrait tap identify the expected revealed mineral; the warning disappears when an authoritative snapshot raises access. Both views pass contrast/type/touch-target checks. The inspector scrolls within the mine viewport when space is limited.

Current checks: 112 core, 15 native, 18 campaign strategy and 30 Bun tests. Frontend production build passes. ARM64 APK rebuilt on c52ce69; packaged ELF and ZIP alignment plus v2/v3 signatures pass. SHA-256: d17593b8a3c99c75ac8a1094252304888c96b7f13e14f4f2ad125ebb34dfabd9. The 30-seed allocation-aware campaign remains in progress.


## Completed campaign cohort and delivery

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

The committed compact report is [docs/balance/shaft-first-campaign.json](balance/shaft-first-campaign.json); [reproduction notes](balance/README.md) explain source provenance and strict outlier calculations. The gate passes against current pacing content.

Final checks remain 112 core, 15 native, 18 campaign strategy and 30 Bun tests. The latest Windows Bun 1.4.0 frontend build also passes with Node absent from process PATH. APK source stays c52ce69 with SHA-256 d17593b8a3c99c75ac8a1094252304888c96b7f13e14f4f2ad125ebb34dfabd9; no runtime code changed during cohort measurement. Physical Android acceptance and the Defender-blocked deep Windows endurance run remain unverified as previously documented.
