# Inkplainer-OS → Inkplainer-RustApp port plan

## Objective

Reimplement Inkplainer as a Windows-native Rust application while preserving the upstream
workflow and output semantics. No HTML/JS/WebView layer should remain in the production app.

## Architecture mapping

| Upstream browser implementation | Rust native replacement |
|---|---|
| DOM + HTML controls | `egui` panels/widgets |
| `<canvas>` 2D rendering | `egui` GPU painting for preview + `image` CPU renderer for export |
| `Image` / browser decoders | `image` crate |
| `requestAnimationFrame` | native frame loop in `eframe::App::update` |
| IndexedDB/local browser storage | native project files + later app-data project index |
| WebCodecs / mp4-muxer | current ffmpeg bridge; later native bundled encoder |
| Browser file picker | `rfd` native Windows dialogs |
| JS global `state` | strongly typed Rust `Project`, `Layer`, `AnimationSettings` |
| animation slots/groups | deterministic per-layer group scheduler |

## Milestones

### M1 — Native editor foundation (implemented in this scaffold)
- Native executable
- Image/text layers
- Layer stack, transforms, opacity, visibility
- Animation order and parallel groups
- Core reveal algorithms
- Project save/load
- PNG/MP4 export

### M2 — Drawing parity
- Port Classic / Adaptive / Morph Shell / Canny+ edge algorithms
- Port internal ink extraction
- Stroke engines: Default, Charcoal, Sketch, Fountain, Blueprint
- Full Outline Fill / Illust Fill / Outline Only behavior
- Coloring styles: Sparse, Filled, Watercolor
- Reveal transitions: Fade, Dissolve, Wipe, Iris, Scanlines

### M3 — Editor parity
- Canvas hit-testing + selection handles
- Aspect-preserving resize handles
- Undo/redo command stack
- Multi-select
- Grid / rectangle / freehand slicer
- Preset manager
- Custom backgrounds/textures
- Hand image library and smooth glide

### M4 — Project/media parity
- Portable `.inkplainer` package with embedded assets
- Auto-save and project browser
- WebM support
- Self-contained video encoding (no system ffmpeg requirement)
- Export progress/cancel worker
- Audio/voice timeline hooks for the larger whiteboard-maker product

### M5 — Performance
- Rayon/worker-thread preprocessing for edge maps
- GPU-assisted image masks where beneficial
- Cached resized textures and edge data
- Incremental export renderer
- Benchmark against upstream at 720p/1080p/1440p
