# Inkplainer-RustApp

Native Windows Rust port of [Inkplainer-OS](https://github.com/NadirWeb-App/Inkplainer-OS), a
free/open-source whiteboard animation editor.

This project intentionally does **not** use Electron, Tauri, a WebView, HTML, or JavaScript at
runtime. The editor is a native Rust executable built with `eframe/egui`, native image processing,
and self-contained Rust video encoders.

## Implemented feature set

- Native Windows desktop editor (`eframe` / `egui`)
- Image layers (PNG/JPG/GIF/WebP first frame)
- Text layers with installed-font selection
- Layer visibility, opacity, stacking, grouping, position and resize handles
- Per-layer animation order; equal order values animate in parallel
- Undo / redo and keyboard playback/edit shortcuts
- Auto-save/restore and project browser
- Portable copied project assets
- Grid, rectangle-drag and freehand slicer workflows
- Upstream whiteboard animation styles, including Chunk Jump, Scanner/Zigzag, Contour,
  Outline Chunks, Outline Fill, Illust Fill, Outline Only, Text Draw, Scribble, Nervous,
  Top Anchor, Human, Animal, Portrait, Vehicle, Building, Landscape, Spiral and Nature
- Classic, Adaptive, Morph Shell and Canny+ outline detection
- Default, Charcoal, Sketch, Fountain and Blueprint stroke styles
- Sparse, Filled and Watercolor coloring modes
- Instant, Fade, Dissolve, Wipe, Iris and Scanlines reveal modes
- Upstream Hand 1/2/3, Pen, Cartoon and Pixel Art sprite library with smooth hand glide
- PNG final-frame export
- Self-contained MP4/H.264 export
- Self-contained WebM/AV1 export
- Background export worker with progress/cancel
- Windows CI with render, persistence and native codec regression tests

See [`PARITY_STATUS.md`](PARITY_STATUS.md) for the audited parity checklist and limitations.

## Video export

MP4 and WebM export do **not** require an installed `ffmpeg` executable.

- MP4 uses the native Rust H.264 pipeline and MP4 muxer.
- WebM uses `rav1e` AV1 encoding and a native WebM/EBML muxer.
- CI smoke tests encode real files and validate the MP4 `ftyp` and WebM EBML container headers.

Available output controls include 720p / 1080p / 1440p presets, FPS and quality.

## Hand sprite library

The six non-Ghost hand styles use the exact PNG sprite assets and `tipX`/`tipY` anchor metadata from
the pinned Inkplainer-OS baseline documented in [`UPSTREAM.md`](UPSTREAM.md). Landscape/portrait and
720p/1080p/1440p variants are selected to match the canvas. The same sprites are used by interactive
preview and offline PNG/video rendering.

During a CI/release build, missing upstream sprites are treated as a build failure. Local source
builds can use a transparent fallback when the network is unavailable.

## Architecture

```text
src/
  main.rs             Native app entrypoint
  app.rs              Editor UI, commands, playback and project workflow
  model.rs            Serializable project/layer/settings model
  assets.rs           Image decoding, textures, edge extraction and runtime hand assets
  hand_assets.rs      Pinned upstream hand metadata/embedded sprite definitions
  animation.rs        Timeline grouping + reveal algorithms
  effects.rs          Edge/detection/stroke processing
  render.rs           Interactive native canvas rendering
  export.rs           CPU frame renderer + PNG frame path
  native_mp4.rs       Self-contained H.264/MP4 export
  native_webm.rs      Self-contained AV1/WebM export
  export_worker.rs    Background export/progress/cancel
  slicer.rs           Grid/rectangle/freehand slicing
  project_io.rs       Save/open/autosave/project library
  font_catalog.rs     Installed font discovery/rendering
  regression_tests.rs CI regression/smoke tests
```

The animation model is shared between interactive preview and offline rendering so reveal timing and
hand motion follow the same project state.

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

## Project format

Projects are JSON documents (`*.inkplainer.json`). Imported image assets are copied into a portable
asset directory next to the saved project so a project is not dependent on the original import
location.

## Upstream and license

This is a derivative/reimplementation of Inkplainer-OS by Nadir. See [`UPSTREAM.md`](UPSTREAM.md)
for the pinned baseline and asset attribution. The upstream project is Apache-2.0 licensed, and this
port retains the required license and attribution.
