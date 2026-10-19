# Inkplainer-OS → Inkplainer-RustApp port plan

## Objective

Reimplement Inkplainer as a Windows-native Rust application while preserving the pinned upstream
workflow and output semantics. No HTML/JS/WebView layer remains in the production runtime.

For the audited baseline and current limitations, see `PARITY_STATUS.md`.

## Architecture mapping

| Upstream browser implementation | Rust native replacement |
|---|---|
| DOM + HTML controls | `egui` panels/widgets |
| `<canvas>` 2D rendering | `egui` GPU painting for preview + `image` CPU renderer for export |
| `Image` / browser decoders | `image` crate |
| `requestAnimationFrame` | native frame loop in `eframe::App::update` |
| IndexedDB/local browser storage | native project files, autosave and project library |
| WebCodecs / mp4-muxer | self-contained Rust H.264/MP4 + AV1/WebM pipelines |
| Browser file picker | `rfd` native Windows dialogs |
| JS global `state` | strongly typed Rust `Project`, `Layer`, `AnimationSettings` |
| animation slots/groups | deterministic per-layer group scheduler |
| browser hand PNGs | exact pinned upstream sprites embedded in the native executable |

## Milestones

### M1 — Native editor foundation — complete
- Native executable
- Image/text layers
- Layer stack, transforms, opacity, visibility
- Animation order and parallel groups
- Core reveal algorithms
- Project save/load
- PNG export

### M2 — Drawing parity — complete for audited baseline
- Classic / Adaptive / Morph Shell / Canny+ edge algorithms
- Internal edge/ink processing
- Default, Charcoal, Sketch, Fountain and Blueprint strokes
- Outline Fill / Illust Fill / Outline Only behavior
- Sparse, Filled and Watercolor coloring
- Fade, Dissolve, Wipe, Iris and Scanlines reveal transitions

### M3 — Editor parity — complete for audited baseline
- Canvas hit-testing and native selection
- Direct resize handles
- Undo/redo history
- Grid / rectangle-drag / freehand slicer
- Presets/settings workflow
- Background styles/textures
- Installed-font selection/rendering
- Pinned upstream hand image library and smooth glide

### M4 — Project/media parity — complete for audited baseline
- Portable copied project assets
- Auto-save/restore and project browser
- Self-contained MP4/H.264 export
- Self-contained WebM/AV1 export
- Background export worker with progress/cancel
- Native codec smoke tests in Windows CI
- Representative pixel-level render regression gate

### M5 — Performance and hardening — ongoing
- Expand representative upstream pixel fixtures across more animation/effect combinations
- Profile edge extraction and CPU frame compositing
- Add worker-thread/Rayon preprocessing where measurements justify it
- Cache expensive resize/edge intermediates where memory trade-offs are acceptable
- Benchmark native preview/export at 720p/1080p/1440p against the pinned upstream baseline
- Continue adding regressions for every discovered visual or behavioral mismatch
