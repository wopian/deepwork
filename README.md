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
cargo test -p mine-core --locked
bun.exe run tauri build --debug --no-bundle
```

The unpackaged Windows executable is `target/debug/deepwork.exe`. Browser-only Vite preview intentionally does not simulate the economy: the Rust backend is authoritative.

## Implemented

- Seeded, persistent pixel-column excavation; three excavation priorities and policies.
- Worker recruitment, housing, conveyor/processing capacity, resource buffers and raw sales.
- 56 mineral feeds, colour-coded field guide, refined products, optional manufacturing and separation recipes.
- Explicit alumina/electrolysis path, abstract mass-conserving recipe units, steel and component production.
- Contracts, resource reserves, power throttling, recovery, research, retirement and megaproject delivery.
- Tactical boosts, offline simulation at half rate up to eight real hours, local saves and backup recovery.
- Pixi world with extended terrain palette, camera pan/zoom/follow, responsive interface and reduced-motion preference.
- Bun-only toolchain, Rust accounting tests and Windows CI.

## Current limits against the full design

This is an early playable implementation, **not the finished campaign**. Remaining work is tracked in `IMPLEMENTATION_STATUS.md`. In particular, excavation uses column frontiers rather than a full branching tunnel/pathfinding system; production advances in one-second batches; offline simulation replays those batches. Rendering shows representative workers and machinery rather than authoritative transport entities. Mobile builds and live visual acceptance are not yet verified.

## Toolchain notes

`vue-tsc` normally intercepts CommonJS file reads to patch TypeScript. Bun bypasses that interception. `scripts/typecheck.cjs` applies Volar's transformation explicitly and runs it under Bun; template checks remain enabled. The checked tool versions and both dependency lockfiles should be committed together.

`./scripts/check-no-node.ps1` temporarily removes Node.js directories from its process PATH and builds the frontend with Bun. It does not alter system settings.

## Content

Recipes use abstract game units, not real chemical ratios. Material identities reference USGS; the proposed economics and use of mineral products are game design.

- https://www.usgs.gov/centers/national-minerals-information-center/minerals-yearbook-metals-and-minerals
- https://www.usgs.gov/programs/mineral-resources-program/science/about-2025-list-critical-minerals
- https://www.usgs.gov/programs/mineral-resources-program/minerals-and-uses-activity
- https://v2.tauri.app/start/frontend/vite/
