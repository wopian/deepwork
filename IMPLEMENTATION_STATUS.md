# Implementation status

## Working foundation

Bun 1.4.0 / Vue 3 / Vite / PixiJS frontend and Tauri 2 / Rust backend are implemented. The game includes a local economic loop, persistent excavation, basic progression, optional industry recipes, research, retirement and native save commands.

## Required before calling the full plan complete

- Replace column excavation with chunked terrain supporting reachable branching tunnels, supports, groundwater and heat gates.
- Model worker roles, route graphs, actual minecarts/trains/lifts, construction and district expansion. Tie each visual actor and building to authoritative state.
- Run foreground simulation at 20 Hz and send deltas; implement offline boundary-event advancement rather than one-second replay.
- Separate sorting and refining queues and make each production-strip metric reflect its own measured flow.
- Introduce site geology and three meaningful site/challenge choices, guaranteed milestone reserves, equipment tiers and material-priced unlocks.
- Complete automatic upgrade reserves/presets, meaningful specialisation, research-rank 3/6/10 features, museum cross-sections and milestones.
- Replace abstract one-input trace recipes with graded eligible-stream recovery; add process residue chemistry abstractions without creating material.
- Implement fresh falling spoil particles, compacted strata, distinct slag/depleted-rock disposal and resource-preserving visual cleanup.
- Balance the first hour and multi-week progression with headless strategy runs and player testing. Current prices are provisional.
- Add full audio mix, quality settings, numeric-format choice and colour-independent terrain patterns.
- Verify mobile lifecycle/import/export, package Android/iOS/macOS/Linux, and test actual touch interaction and target FPS.
- Run 30-minute memory/visual stress test, restart/import failure scenarios and concurrent-instance save protection.

## Validation scope

Rust tests cover deterministic replay, save round-trip, command deduplication, offline equivalence/cap, recipe references and material accounting. Vue templates and production frontend compile under Bun. Windows Tauri compilation and a debug application build have passed. Live UI inspection was blocked by the computer-use helper's workspace-URI error; no visual or mobile acceptance is claimed.
