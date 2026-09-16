# Implementation status

## Playable implementation

- Bun 1.4.0, Vue 3, plain Vite, PixiJS 8 WebGL, Tauri 2 and an authoritative Rust simulation.
- Fixed 20 Hz production, five-Hz state delivery and changed terrain chunks. Seed and price strings retain integer precision across IPC.
- Sparse 64×64 excavation masks, reachable pit/shaft/branch frontiers, depth hardness, drill tiers, automatic support construction and groundwater/heat gates.
- Six worker roles with population-preserving priorities, recruitment and housing. Representative sprites respect visual budgets.
- Cached deterministic routes through excavated cells, with carrying, wheelbarrows, tunnel carts/trains, shaft lifts and surface conveyors. Cargo transit and full buffers preserve material.
- Separate sorting/refining queues, raw/refined sales, power throttling and measured production blockers.
- 56 mineral feeds, 41 recipes, eligible finite trace residues, aluminium/alumina distinction, alloys and advanced components. Startup checks reject inaccessible recipe/module cycles.
- Tailings, slag and depleted rock ledgers, reclamation, free disposal and bounded visual spoil particles.
- Material-priced upgrades, pinned reserves, machine versus feed-line capacity forecasts, renewable premium contracts and campaign-unique research rewards.
- Permanent refined-product collection, including trace products and components after sale/retirement. Cargo policies favour preferred minerals with starvation-free fair slots.
- Three geological profiles, optional hard-rock/long-haul challenges, three site specialisations, research branches, rank 3/6/10 equipment, presets, museum cross-sections, milestones and headquarters megaproject.
- Tactical abilities; foreground-only cooldowns. Freight priority accelerates transit; furnace overdrive raises power demand.
- Capped half-rate offline progress, once-only checkpoints, discoveries/blocker reports and exact fast-forward through stalled intervals and empty gaps before excavation/cargo events.
- Versioned atomic saves, previous-good recovery, exclusive process locks, transactional commands/imports, explicit retirement quotes and portable desktop saves.
- Responsive desktop/portrait controls, pan/zoom/depth and crew follow, district shortcuts, reduced motion, visual budgets, full/compact/scientific numbers, mineral pattern marks and opt-in mixed machinery/interface audio.

## Remaining against the full plan

- Aggregate **active** offline production between depletion/buffer events. Filled processing pipelines still use deterministic ticks. Empty intervals skip to the next excavation/arrival event; stalled intervals skip to final feedback.
- Expand route capacity into independently selectable segment buffers/express priorities. Current cargo handoffs use segment travel times and shared loading/arrival capacity.
- Expand the initial district artwork further. Current presentation includes growing housing, shaft headframes, rail depots, shared processing-module halls, reclamation machinery and ten-level tier badges.
- Validate multi-week campaign pacing, all geological profiles and endgame strategies with broader headless strategies and human playtests. Current endgame amounts/prices remain provisional.
- Verify mobile lifecycle on hardware, real touch play and Android builds/devices. Native Windows save download/import and suspend/resume controls pass integration checks. Android tools are not configured in the current Windows PATH/environment. macOS/Linux/iOS packaging/device checks remain separate.
- Repeat full-duration stress testing after further simulation/render changes; distinguish JS heap/native working-set samples from total GPU/WebView process memory.
- Validate installation/uninstallation and clean-machine WebView2 prerequisites. A successful installer build alone does not prove installation behavior.

## Local evidence

56 Rust tests and six Bun tests pass locally. Rust tests cover deterministic replay, exact offline parity, clock cap/rollback, accounting, depletion/reclamation, save recovery, command deduplication, retirement quotes, reachable content, support construction, route shortcuts, imports and crew conservation. Bun tests cover catalogue routes, numeric precision, transport interpolation and bounded particle settling.

Windows debug gameplay checks use the actual WebView2 application with isolated saves: live production, purchase, disk checkpoint, reload, tab navigation and corrupt-import rejection. Desktop and 390-pixel portrait browser checks also exercise audio/quality/number controls. This is not mobile-device validation.

`cargo run -p mine-core --example balance -- depth 0` and `-- bulk 0` report a fixed construction-only strategy; they are not an optimal upgrade bot. With segment transport and doubled baseline sorting/refining capacity, Copper Ridge depth policy reached conveyor at 4:15, furnace at 11:47, shaft at 30:40 and retirement at 36:55. The same strategy reached retirement at 35:30 on Crystal Basin and 51:56 on Deep Granite. These cross-site runs caught automatic steelmaking consuming pinned shaft iron; recipes now protect upgrade reserves, and CI runs all three profiles. An earlier bulk-policy run reached conveyor at 3:10 and furnace at 9:38, but remained below retirement depth after four hours without additional throughput upgrades. Re-run after economic/transport changes rather than treating these as fixed guarantees.

Use `bun.exe run test:native test-results 1800` for a 30-minute isolated 1,000-worker stress scenario. Add `release` as the last argument to test a copied release executable in portable mode with development runtimes excluded from its PATH. The harness records retained visual entity counts, sampled JS heap, native working set and rendering diagnostics.

### Recorded stress evidence

A 30-minute native debug baseline with 1,000 simulated workers completed: visible workers stayed capped at 250, particles at 40, sampled JS heap ranged 14.3–80.9 MB and the game process working set 33.9–35.2 MiB. This preceded later route/crew/render changes. Its old instantaneous ticker FPS field is not valid evidence of sustained rendering FPS.

A subsequent one-minute release run used portable data and excluded Bun/Node from the child process PATH. Whole-second rendered-frame samples measured 58–59 FPS with 250 visible workers. This does not establish mobile performance or total WebView/GPU memory usage. Windows NSIS builds succeeded; installation/uninstallation remains untested.
