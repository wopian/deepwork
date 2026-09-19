# Shaft redesign — 2026-09-19

The signed-coordinate expansion, whole-vein orders, contextual desktop/mobile workshop, processing telemetry and fast-opening retune are implemented. Current saves are version 9 / generator 5; previous saves remain exportable archives.

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

The `offline_replay` example compares an exported campaign save under capped offline advancement and exact fixed steps. A 3.6 km save with 4,856 passages matched complete serialized state after eight real hours / four credited simulation hours, excluding only the report and checkpoint timestamp. Both paths excavated 30,436 cells and earned 6,098 credits. Local computation took 116.5 seconds offline and 115.0 seconds stepped; this proves accounting parity, not acceptable Android catch-up latency.
