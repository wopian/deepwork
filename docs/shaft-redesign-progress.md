# Shaft redesign checkpoint — 2026-09-19

This is a playable checkpoint, not completion of the full redesign plan.

## Implemented

- Supported underground development starts at the surface, without buying a shaft or spending iron. Shaft purchases improve later access. Starting credits cover one worker and the starter furnace.
- Cooldown powers removed from the core, saves, interface and campaign strategy. Commodity priorities and surveyed ore target selection remain.
- Deterministic deposit identities and saved survey stages. Survey level 1 reveals a facing portion of a deposit; level 2 reveals its unmined cells. Private survey anchors are stripped from IPC snapshots.
- Hover/tap mineral inspection and a command to direct crews toward a revealed, unmined ore cell through normal supported passage planning.
- Anchored pinch/wheel zoom, close inspection, mine overview, initial crew-follow camera and touch gesture separation.
- Larger, higher-contrast interface text, collapsible equipment controls and processing workshop with separate raw-feed, intake, product and reserve quantities. Processing remains continuous.
- Offline summaries have persistent interval identities. Lifecycle work runs through a serial worker rather than the native event handler. Catch-up commits before publication; queued requests from an earlier reset epoch are rejected. Backwards clocks grant no elapsed time and still resume play.
- Save version 8 / generator 4. Older saves use the existing archive/export and fresh-start flow; preferences remain separate.
- `scripts/android-test.cmd` builds an ARM64 test APK with Windows Bun 1.4.0. Its narrowly checked symlink fallback copies the successfully compiled native library before Gradle packaging. Output uses the local Android debug certificate, not a distribution signing key.

## Measured checks

- 96 Rust core tests, 11 native persistence/lifecycle tests, 23 Bun tests pass.
- Vue/TypeScript validation and production asset build pass with Bun 1.4.0.
- Thirty deterministic starter seeds, five simulated minutes each, with the guide buying an affordable furnace: all pass. First sale 17–28 seconds; refined iron 17–199 seconds (median 24); first horizontal branch 3 seconds.
- These opening runs are not adverse-purchase-order tests or complete campaign simulations.
- Built-asset browser checks pass desktop and portrait layout, navigation and page-error checks. They are illustrative browser previews, not Android device evidence.
- ARM64 APK signature verification passes v2/v3 and 16 KB ZIP alignment checks. This is not an ELF alignment or hardware performance claim.

## Remaining plan work

1. Replace width-dependent indexes with signed horizontal cell/chunk coordinates. Current underground still spans 512 cells / 128 metres; horizontal streaming is **not implemented**.
2. Continue whole-deposit priority after the selected ore cell is removed. Current targeting persists a cell, not a complete deposit-exhaustion order. Improve the facing reveal into a continuous contour rather than a proximity slice.
3. Complete contextual worker/machine/route inspection, mobile bottom-sheet behavior, per-feed processing rates, effective committed reserves and justified upgrade completion estimates. Validate all contrast and font-size cases, including canvas labels.
4. Exercise contradictory native/WebView lifecycle ordering and physical Android background, screen-lock, process-death and recreation scenarios. Automated native tests do not establish device correctness.
5. Re-author old minute pacing windows and baseline purchase strategies, test adverse purchase/sale orders, and run 30 complete campaign seeds. The previous 4–6-week results do not validate this opening or geometry.
6. Run the expanded-terrain stress case after horizontal streaming exists: frame times, bounded caches, memory and save size. Complete Windows gameplay and actual Android touch/FPS acceptance.

No claim is made that the entire campaign or full redesign is finished.
