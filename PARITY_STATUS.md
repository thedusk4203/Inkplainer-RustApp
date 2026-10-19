# Inkplainer-OS → RustApp parity audit

This file tracks the native Rust port against the pinned Inkplainer-OS baseline documented in `UPSTREAM.md`.

A checked item means the implementation is present in source and has an applicable regression/test or a directly audited code path. A release still requires the latest Windows CI head to complete `cargo check -D warnings`, the full test suite and the release build successfully.

## Core editor

- [x] Native Rust/egui editor; no WebView/HTML/JavaScript runtime
- [x] Inkplainer-style top/left/center/right/bottom native layout
- [x] Image layers
- [x] Text layers
- [x] Visibility, opacity, stack order, rename, position and size
- [x] Per-layer animation order
- [x] Equal animation-order values run in parallel
- [x] Group metadata editing and project round-trip preservation
- [x] Native drag positioning
- [x] Resize handles directly on canvas
- [x] Undo / redo
- [x] Keyboard playback/edit shortcuts
- [x] Installed-font picker, custom family entry and native text rendering
- [x] Text alignment, color, line-height and letter-spacing controls
- [x] New / Open / Save / Save As / Restore Autosave workflows exposed in native UI

## Timing

- [x] Reveal Speed range 1–100 affects actual rendered layer progress
- [x] Default Reveal Speed preserves the baseline timing profile
- [x] Hand Speed range 1–20 independently maps the hand along the active path
- [x] Preview and offline export share the same deterministic hand-timing function
- [x] Speed functions preserve 0/1 endpoints and sanitize invalid/out-of-range values
- [x] Cross-module regression verifies faster Reveal Speed exposes more pixels at the same timestamp

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
- [x] Outline color / thickness / opacity controls
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

## Preview / offline-render consistency

- [x] Iris preview uses radial geometry matching the CPU-render semantics
- [x] Watercolor pass opacity matches preview and CPU renderer
- [x] Preview text honors line height and letter spacing
- [x] Preview stroke thickness scales with canvas display scale
- [x] Pattern/background colors and spacing are aligned with CPU export settings
- [x] Preview and offline render use the same animation progress and hand-motion mapping

## Hand system

- [x] Ghost / Hand 1 / Hand 2 / Hand 3 / Pen / Cartoon / Pixel Art choices
- [x] Exact upstream hand sprite URLs pinned to the audited upstream commit
- [x] Persistent Cargo cache for pinned sprites
- [x] curl + PowerShell download fallback with retries and PNG validation
- [x] CI fails instead of silently releasing missing/fallback sprites
- [x] 720p / 1080p / 1440p and portrait sprite variants
- [x] Upstream tipX/tipY anchoring
- [x] Smooth-hand toggle affects the actual hand trajectory
- [x] Same hand assets/trajectory model in interactive preview and offline PNG/video rendering

## Slicer

- [x] Grid mode (1–8 columns / rows)
- [x] Rectangle-region backend
- [x] Rectangle direct-drag UI
- [x] Freehand polygon backend/UI
- [x] Preserve source layer animation/order/group settings
- [x] Freehand polygon works clockwise and counter-clockwise
- [x] Slice images are written to persistent app cache rather than system temp

## Project workflow

- [x] Project JSON save/open
- [x] Portable copied asset directory next to explicit saves
- [x] Autosave and project-library snapshots copy/stage image assets
- [x] Staged assets are compared by content before an existing copy is reused
- [x] Failed new image imports are removed instead of persisting zombie layers
- [x] Autosave is repaired immediately if a failed import is rolled back in the same frame
- [x] Project browser includes asset-directory bytes in storage totals
- [x] Deleting a library project removes its staged asset directory

## Export

- [x] PNG export
- [x] Self-contained MP4/H.264 export (no ffmpeg executable required)
- [x] Self-contained WebM/AV1 export (no ffmpeg executable required)
- [x] Canvas aspect ratio is preserved for landscape, portrait, square and arbitrary sizes
- [x] Codec output dimensions are normalized to even values
- [x] Extreme output dimensions are bounded without stretching
- [x] 720p / 1080p / 1440p-style controls
- [x] Export quality/FPS controls
- [x] Background export worker
- [x] Export progress and cancel
- [x] Cancelled exports remove stale/partial output files

## Regression coverage

- [x] Animation-order / same-order parallelism tests
- [x] Group metadata project round-trip test
- [x] Reveal-speed monotonicity/default/endpoint tests
- [x] Hand-speed monotonicity/default/endpoint/range tests
- [x] Reveal-region bounds tests
- [x] Deterministic reveal-geometry tests
- [x] Representative pixel-level Scanner fixture derived from upstream behavior
- [x] Project JSON/file round-trip tests
- [x] Portable asset survival test
- [x] Same-size/stale staged-asset content comparison test
- [x] Aspect-resolution tests for landscape, portrait, square, arbitrary and extreme ratios
- [x] MP4 top-level box / AVC configuration structural validation
- [x] MP4 visual sample entry dimension regression
- [x] WebM EBML / Segment / Tracks / Cluster / AV1 structural validation
- [x] Video export cancellation cleanup regression

## Current parity statement

The source-level audit for the pinned upstream baseline is functionally complete for the items above. This does **not** claim universal bit-for-bit frame identity with the browser implementation: font rasterization, GPU/CPU sampling, operating-system rendering and untested combinations can still produce visual differences.

The latest repository head must pass Windows CI before it should be tagged or distributed as a validated release. Representative pixel-level and cross-module regression tests are retained as CI gates and should be expanded whenever a mismatch is found.
