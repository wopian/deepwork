# Warm industrial visual design

Deepwork uses a persistent mine viewport with a compact game HUD. Build, Crew, Industry, Minerals and More expose all existing controls. Desktop uses a 380 px drawer; narrower frames use a bottom sheet. Drawers scroll independently and restore keyboard focus when dismissed. Camera controls live in one menu.

Campaign guidance reads authoritative upgrade prerequisites, purchase blockers, material quantities and research. Pinned equipment takes priority. Guidance never creates a purchase, reserve or crew order. View reveals the matching upgrade. Headquarters progress includes both research and required components. Guidance and tutorial dismissal remain device preferences; blocked storage does not prevent play.

## Artwork and state

Procedural pixel artwork adds warm camp windows, a braced headframe, distinct processing halls, shaded mineral fabrics, tunnel floors, readable support columns, work lamps, and role-specific crew equipment. Public ore masks still define vein boundaries. Excavation masks define cleared terrain; unfinished supports use committed construction progress. Worker and cargo movement retain fractional positions and authoritative travel timing.

Discovery notices compare committed states. Loading, imports, catch-up and reconciliation establish a baseline without celebrating old discoveries. Reduced motion disables decorative animation and transitions. Idle replay continues to show excavation and construction without cargo movement.

The terrain texture ceiling remains 768. Worker budgets remain 100/250 and the shared cargo/particle budget remains 600/2,000 for low/high quality. This change adds no GPU textures for artwork, no Rust state fields and no save migration. Existing staged recovery work remains separate.

## Verification

```powershell
bun.exe test
bun.exe run build
bun.exe scripts/browser-check.ts test-results/presentation/browser --built
cargo build -p deepwork --release --features tauri/custom-protocol -j 2 --locked
bun.exe scripts/presentation-native.ts --fixture test-results/natural-veins-final/native-fixture.json
```

The native harness uses a retained generator-6, version-12 fixture. Supply another compatible JSON checkpoint with `--fixture` when that local artifact is unavailable. Native acceptance always copies the executable into a temporary directory and uses isolated portable saves. WebView2 launch requires an unrestricted process environment on this machine; restricted launches stalled before window creation.

Browser acceptance checks all 11 management panels, persistent canvas ownership, contrast, touch targets, Escape/focus restoration, panel layouts, blocked storage, reduced motion, and 320×568, 390×844, 640×360, 836×470, 1031×580 and 844×390 layouts. Host overlays use `--host-overlay-left` / `--host-overlay-right` insets; the portrait check reserves 46 px.

Native acceptance checks starter and mature mines, direct goal navigation, panels, touch layouts, reduced motion, timelapse command locks, Skip and a 1,000-worker fixture. Screenshots and measured samples remain in `test-results/presentation/`. Matching mature-fixture samples report 21.2 ms p95 before and after the redesign. The 1,000-worker check renders 250 workers, 28 terrain textures and 22 moving entities at 21.2 ms p95. Retained JavaScript heap after forced GC is 8,143,480 bytes for baseline and 10,177,396 bytes for redesign; the richer presentation costs roughly 2 MB of retained heap. Original uncollected samples remain in the evidence. Short native samples are renderer regression evidence, not sustained FPS qualification. No campaign cohort or `travel-progress.ts` rerun is needed for these frontend changes.

## Potential Poki launch

Assets and libraries are local. The layout supports small desktop frames, portrait and landscape without requiring fullscreen. Browser preview remains illustrative. A playable Poki release still needs browser simulation and persistence, Poki SDK events, platform testing and publishing assets. This visual change does not claim Poki release readiness.
