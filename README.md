# Inkplainer-RustApp

Native Windows Rust port of [Inkplainer-OS](https://github.com/NadirWeb-App/Inkplainer-OS),
a free/open-source whiteboard animation editor.

This project intentionally does **not** use Electron, Tauri, a WebView, HTML, or JavaScript.
The editor runs as a native Rust executable using `eframe/egui` and CPU image processing.

## Current implementation

The initial port provides a usable native editor foundation:

- Native Windows desktop window (`eframe`/`egui`)
- Image layers (PNG/JPG/GIF/WebP first frame)
- Text layers
- Layer visibility, opacity, stacking, position and size
- Per-layer animation order; equal order values animate in parallel
- Playback timeline
- Native implementations of:
  - Chunk Jump
  - Scanner + Zigzag
  - Contour
  - Outline Chunks
  - Outline Fill
  - Illust Fill (currently shares the outline/reveal pipeline; coloring pass is next)
  - Outline Only
  - Text Draw
  - Human / Animal / Portrait / Vehicle / Building / Landscape / Spiral priority reveals
- Project save/open as JSON (`*.inkplainer.json`)
- PNG final-frame export
- MP4 export through installed `ffmpeg`
- Windows CI workflow

## Port status vs upstream

| Area | Status |
|---|---|
| Native Rust editor shell | Implemented |
| Image/text layers | Implemented |
| Layer animation order / parallel groups | Implemented |
| Chunk/Scanner/Specialized styles | Implemented |
| Contour/Outline base pipeline | Implemented |
| Full upstream outline algorithms (Classic/Adaptive/Morph/Canny+) | Planned |
| Stroke styles (charcoal/sketch/fountain/blueprint) | Planned |
| Full Illust Fill coloring styles | Planned |
| Slicer (grid/rectangle/freehand) | Planned |
| Hand sprite library + smooth hand glide | Planned |
| Background texture parity | Partial |
| Project asset embedding | Planned (currently stores image paths) |
| PNG export | Implemented |
| MP4 export | Implemented via ffmpeg |
| WebM export | Planned |
| Auto-save/project browser | Planned |
| Undo/redo | Planned |

## Architecture

```text
src/
  main.rs         Native app entrypoint
  app.rs          Editor UI + commands + playback
  model.rs        Serializable project/layer/settings model
  assets.rs       Image decoding, GPU texture upload, edge extraction
  animation.rs    Timeline grouping + reveal algorithms
  render.rs       Interactive native canvas rendering
  export.rs       CPU frame renderer + PNG/MP4 pipeline
  project_io.rs   Project JSON persistence
```

The renderer intentionally keeps animation algorithms independent from the UI so the same
reveal logic can be reused by interactive playback and offline video rendering.

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

### MP4 export

MP4 export currently calls `ffmpeg` from `PATH`:

```powershell
ffmpeg -version
```

If that command is unavailable, PNG export still works. A later milestone will bundle or
replace the external encoder so video export is self-contained.

## Project format

Projects are JSON documents. Image layers currently retain absolute/local file paths to source
assets. Moving the source files can therefore produce a missing-asset warning. The next project
format revision will embed/copy assets into a portable project package.

## Upstream and license

This is a derivative/reimplementation of Inkplainer-OS by Nadir. See [UPSTREAM.md](UPSTREAM.md).
The upstream project is Apache-2.0 licensed, and this port retains that license and attribution.
