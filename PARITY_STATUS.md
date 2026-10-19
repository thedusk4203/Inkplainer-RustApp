# Inkplainer-OS → RustApp parity audit

This file tracks behavior against the upstream Inkplainer-OS implementation. A checkbox is only
marked complete when the Rust implementation exists; build/test status is tracked separately by
Windows CI.

## Core editor

- [x] Native Rust/egui editor; no WebView/HTML/JavaScript runtime
- [x] Image layers
- [x] Text layers
- [x] Visibility, opacity, stack order, position and size
- [x] Per-layer animation order
- [x] Equal animation order runs in parallel
- [x] Group metadata
- [x] Native drag positioning
- [x] Undo / redo
- [x] Keyboard playback/edit shortcuts

## Animation styles

- [x] Chunk Jump
- [x] Scanner + zigzag
- [x] Contour
- [x] Outline Chunks
- [x] Outline Fill
- [x] Illust Fill
- [x] Outline Only
- [x] Text Draw
- [x] Scribble
- [x] Nervous
- [x] Top Anchor
- [x] Human
- [x] Animal
- [x] Portrait
- [x] Vehicle
- [x] Building
- [x] Landscape
- [x] Spiral
- [x] Nature

## Drawing system

- [x] Classic detection
- [x] Adaptive detection
- [x] Morph Shell detection
- [x] Canny+ detection
- [x] Color Region Outlines
- [x] Real Image Edges
- [x] Default stroke
- [x] Charcoal stroke
- [x] Sketch stroke
- [x] Fountain stroke
- [x] Blueprint stroke
- [x] Sparse coloring
- [x] Filled coloring
- [x] Watercolor coloring
- [x] Instant reveal
- [x] Fade reveal
- [x] Dissolve reveal
- [x] Wipe reveal
- [x] Iris reveal
- [x] Scanlines reveal

## Slicer

- [x] Grid mode (1–8 columns / rows)
- [x] Rectangle-region backend
- [x] Freehand polygon backend
- [x] Preserve source layer animation/order/group settings
- [x] Freehand polygon works clockwise and counter-clockwise

## Project / export

- [x] Project JSON save/open
- [x] Portable copied asset directory next to project
- [x] Auto-save/restore
- [x] PNG export
- [x] MP4 export
- [x] WebM export
- [x] 720p / 1080p / 1440p controls
- [x] Export quality/FPS controls
- [ ] Self-contained video encoder (current MP4/WebM path uses ffmpeg from PATH)

## Remaining UX/visual parity work

- [ ] Distinct Hand 1/2/3/Cartoon visuals matching the upstream hand library
- [ ] Smooth-hand toggle must affect actual hand trajectory (model/UI already present)
- [ ] Rectangle slicer direct-drag UI (backend currently complete; UI also supports numeric regions)
- [ ] Full project browser with multiple auto-saved projects and storage-used display
- [ ] Arbitrary font picker/rendering parity (font family is persisted; native renderer currently uses system/default fonts)
- [ ] Resize handles directly on canvas
- [ ] Export progress/cancel worker
- [ ] Pixel-level regression comparison against representative upstream renders

## Definition of “100%”

Do not call this port 100% feature/visual parity until every remaining item above is implemented and
Windows CI plus representative render regression tests pass.
