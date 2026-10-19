# Inkplainer-RustApp

Native Windows Rust port of [Inkplainer-OS](https://github.com/NadirWeb-App/Inkplainer-OS), a free/open-source whiteboard animation editor.

This project intentionally does **not** use Electron, Tauri, a WebView, HTML, or a JavaScript runtime. The editor is a native Rust executable built with `eframe/egui`, native image processing, and self-contained Rust video encoders.

## Implemented feature set

- Native Windows desktop editor (`eframe` / `egui`) with an Inkplainer-style three-panel layout
- Image and text layers with visibility, opacity, stacking, grouping, position, resize handles, rename and animation order
- Installed-font picker, custom font family entry, alignment, color, line-height and letter-spacing controls
- Per-layer Reveal Speed and Hand Speed timing; equal animation-order values run in parallel
- Undo / redo, keyboard playback/edit shortcuts, project browser and restoreable autosave
- Portable saved projects and autosave snapshots with copied image assets
- Grid, rectangle-drag and freehand slicer workflows backed by a persistent app cache
- Upstream whiteboard animation styles: Chunk Jump, Scanner/Zigzag, Contour, Outline Chunks, Outline Fill, Illust Fill, Outline Only, Text Draw, Scribble, Nervous, Top Anchor, Human, Animal, Portrait, Vehicle, Building, Landscape, Spiral and Nature
- Classic, Adaptive, Morph Shell and Canny+ outline detection
- Default, Charcoal, Sketch, Fountain and Blueprint stroke styles
- Sparse, Filled and Watercolor coloring modes
- Instant, Fade, Dissolve, Wipe, Iris and Scanlines reveal modes
- Upstream Hand 1/2/3, Pen, Cartoon and Pixel Art sprite library with smooth hand glide
- PNG final-frame export
- Self-contained MP4/H.264 and WebM/AV1 export; no installed `ffmpeg` executable required
- Aspect-preserving 720p / 1080p / 1440p-style export sizing, FPS and quality controls
- Background export worker with progress/cancel
- Regression coverage for timing, reveal geometry, persistence, portable assets, representative pixels, codec containers, cancellation and encoded MP4 dimensions

See [`PARITY_STATUS.md`](PARITY_STATUS.md) for the audited parity checklist and limitations.

## Timing model

Reveal and hand timing are independent per-layer settings.

- **Reveal Speed (1–100)** shapes how quickly the selected layer exposes image/stroke content inside its timeline slot.
- **Hand Speed (1–20)** independently maps the hand along the active drawing path.
- Default values preserve the baseline timing profile.
- Preview and offline rendering share the same hand-timing function.

The project duration remains the total video duration; per-layer speeds change progress inside each animation slot rather than silently changing the requested output duration.

## Video export

MP4 and WebM export do **not** require an installed `ffmpeg` executable.

- MP4 uses the native Rust H.264 pipeline and MP4 muxer.
- WebM uses `rav1e` AV1 encoding and a native WebM/EBML muxer.
- Export dimensions are resolved from the current canvas aspect ratio and forced to codec-safe even dimensions.
- CI regression tests validate MP4 box structure and H.264 sample metadata plus WebM EBML/track/cluster structure.

## Hand sprite library

The six non-Ghost hand styles use the exact PNG sprite assets and `tipX`/`tipY` anchor metadata from the pinned Inkplainer-OS baseline documented in [`UPSTREAM.md`](UPSTREAM.md).

`build.rs` retrieves the immutable pinned files on first use, validates that the downloads are PNGs, and stores them in a persistent Cargo cache keyed by the upstream commit. It supports `curl` and PowerShell with retries. CI caches that directory between builds and deliberately fails instead of shipping a missing sprite. For an offline build, pre-populate a directory with the required files and set `INKPLAINER_HAND_ASSET_DIR` to it.

## Architecture

```text
src/
  main.rs             Native app entrypoint and runtime safety wrapper
  app.rs              Editor commands, playback and project workflow
  app/native_ui.rs    Inkplainer-style native egui presentation layer
  model.rs            Serializable project/layer/settings model
  timing.rs           Shared deterministic hand-motion timing
  animation.rs        Timeline grouping, reveal-speed mapping and reveal algorithms
  assets.rs           Image decoding, textures, edge extraction and runtime hand assets
  hand_assets.rs      Pinned upstream hand metadata/embedded sprite definitions
  effects.rs          Edge/detection/stroke processing
  render.rs           Interactive native canvas rendering
  export.rs           CPU frame renderer + PNG path
  export_scale.rs     Aspect-preserving target-resolution mapping
  native_mp4.rs       Self-contained H.264/MP4 export
  native_webm.rs      Self-contained AV1/WebM export
  export_worker.rs    Background export/progress/cancel
  slicer.rs           Grid/rectangle/freehand slicing
  project_io.rs       Save/open/autosave/project library + portable assets
  font_catalog.rs     Installed font discovery and parsed-font cache
  regression_tests.rs Core regression/smoke tests
  audit_tests.rs      Cross-module regressions for audited production bugs
```

## Build on Windows

Install the latest stable Rust toolchain using rustup, then:

```powershell
cargo run --release
```

To create an optimized executable:

```powershell
cargo build --release
```

The executable will be at:

```text
target\release\inkplainer-rustapp.exe
```

A clean build needs access to the pinned upstream hand assets unless `INKPLAINER_HAND_ASSET_DIR` is pre-populated. Subsequent builds reuse the persistent cache.

## Project format

Projects are JSON documents (`*.inkplainer.json`). Imported and sliced image assets are staged into portable asset directories for explicit saves, autosaves and project-library snapshots, so restored projects do not depend on transient source or system-temp paths.

## Upstream and license

This is a derivative/reimplementation of Inkplainer-OS by Nadir. See [`UPSTREAM.md`](UPSTREAM.md) for the pinned baseline and asset attribution. The upstream project is Apache-2.0 licensed, and this port retains the required license and attribution.
