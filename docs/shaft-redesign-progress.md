# Shaft redesign — 2026-09-19

The signed-coordinate expansion, whole-vein orders, contextual desktop/mobile workshop, processing telemetry and fast-opening retune are implemented. Current saves are version 9 / generator 5; previous saves remain exportable archives.

See [implementation status](../IMPLEMENTATION_STATUS.md) for current checks, measured opening results, campaign/endurance status and remaining physical-device acceptance. That document supersedes the earlier fixed-width checkpoint and its incomplete-feature list.

## Final endurance and direct ore interaction

The third 30-minute Windows endurance run passed: median 58 FPS, peak JavaScript heap 80.1 MiB, native working set 39.4 MiB, native private memory 12.1 MiB, 29 resident chunks, 333,578-byte save. Measured simulation/wall time was 1,819.3/1,819.293 seconds. No gameplay commands occurred during measurement. Native memory excludes the WebView and GPU processes.

A separate packaged-app test uses actual mouse/touch drags and clicks to inspect revealed ore, move toward inspector controls and set whole-vein orders. Both modes reached the same authoritative deposit identity. Clicked inspection stays pinned; a mined anchor remains valid when its deposit still contains revealed unmined ore. Rust core checks now total 107. Physical Android acceptance remains separate.
