# Inkplainer-OS → RustApp parity audit

This file tracks the native Rust port against the pinned Inkplainer-OS baseline documented in
`UPSTREAM.md`. A checkbox is marked complete when the implementation exists and is covered by the
current Windows CI build/test gate where practical.

## Core editor

- [x] Native Rust/egui editor; no WebView/HTML/JavaScript runtime
- [x] Image layers
- [x] Text layers
- [x] Visibility, opacity, stack order, position and size
- [x] Per-layer animation order
- [x] Equal animation order runs in parallel
- [x] Group metadata
- [x] Native drag positioning
- [x] Resize handles directly on canvas
- [x] Undo / redo
- [x] Keyboard playback/edit shortcuts
- [x] Arbitrary installed-font picker and native text rendering

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

## Hand system

- [x] Ghost / Hand 1 / Hand 2 / Hand 3 / Pen / Cartoon / Pixel Art choices
- [x] Exact upstream hand sprite library embedded from the pinned baseline
- [x] 720p / 1080p / 1440p and portrait sprite variants
- [x] Upstream tipX/tipY anchoring
- [x] Smooth-hand toggle affects the actual hand trajectory
- [x] Same hand sprites/trajectory in interactive preview and offline PNG/video rendering

## Slicer

- [x] Grid mode (1–8 columns / rows)
- [x] Rectangle-region backend
- [x] Rectangle direct-drag UI
- [x] Freehand polygon backend/UI
- [x] Preserve source layer animation/order/group settings
- [x] Freehand polygon works clockwise and counter-clockwise

## Project workflow

- [x] Project JSON save/open
- [x] Portable copied asset directory next to project
- [x] Auto-save/restore
- [x] Project browser with multiple projects and storage-used display

## Export

- [x] PNG export
- [x] Self-contained MP4/H.264 export (no ffmpeg executable required)
- [x] Self-contained WebM/AV1 export (no ffmpeg executable required)
- [x] 720p / 1080p / 1440p controls
- [x] Export quality/FPS controls
- [x] Background export worker
- [x] Export progress and cancel
- [x] MP4/WebM smoke tests generate real containers in Windows CI

## Regression coverage

- [x] Animation-order/group regression tests
- [x] Reveal-region bounds tests
- [x] Deterministic reveal-geometry tests
- [x] Project JSON/file round-trip tests
- [x] Representative pixel-level Scanner fixture derived from upstream behavior
- [x] Native MP4/WebM container smoke tests

## Current parity statement

The audited functional parity checklist for the pinned upstream baseline is complete. This does not
claim that every possible frame is bit-for-bit identical to the browser implementation: font
rasterization, GPU/CPU sampling, operating-system rendering, and untested combinations can still
produce visual differences. Representative pixel-level regression tests are therefore retained as
a CI gate and should be expanded whenever a mismatch is found.
