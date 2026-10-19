# Upstream attribution

Inkplainer-RustApp is a native Rust reimplementation/derivative of:

- **Inkplainer-OS** by Nadir
- Upstream repository: https://github.com/NadirWeb-App/Inkplainer-OS
- Port baseline commit: `760af54d3c7e0cf3a846fdf3c7bbe278c9f6df3e` (2026-09-03)
- Upstream license: Apache License 2.0
- Upstream copyright: Copyright 2026 Nadir

The Rust port preserves the upstream product model (layers, per-layer animation settings,
animation ordering/groups, whiteboard reveal styles, local-first project workflow) while replacing
the browser/Canvas/WebCodecs implementation with a native desktop architecture.

## Upstream assets reused verbatim

The built-in Hand 1, Hand 2, Hand 3, Pen, Cartoon and Pixel Art PNG sprite families are copied from
the pinned upstream commit above. This includes the 720p, 1080p and 1440p landscape/portrait files
under upstream `images/hand*.png`. Their upstream `tipX`/`tipY` placement metadata is also preserved.

`build.rs` stages those exact pinned assets into Cargo's build output and `src/hand_assets.rs` embeds
them in the native executable. GitHub Actions/release builds fail if a required upstream sprite
cannot be retrieved, preventing release artifacts from silently substituting a fallback image.

All Rust source implementations in this repository are new native implementations unless a file or
section explicitly states otherwise. The reused upstream hand assets remain covered by the upstream
Apache-2.0 license and attribution.
