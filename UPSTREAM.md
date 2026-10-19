# Upstream attribution

Inkplainer-RustApp is a native Rust reimplementation/derivative of:

- **Inkplainer-OS** by Nadir
- Upstream repository: https://github.com/NadirWeb-App/Inkplainer-OS
- Port baseline commit: `760af54d3c7e0cf3a846fdf3c7bbe278c9f6df3e` (2026-09-03)
- Upstream license: Apache License 2.0
- Upstream copyright: Copyright 2026 Nadir

The Rust port preserves the upstream product model (layers, per-layer animation settings, animation ordering/groups, whiteboard reveal styles and local-first project workflow) while replacing the browser/Canvas/WebCodecs implementation with a native desktop architecture.

## Upstream assets reused verbatim

The built-in Hand 1, Hand 2, Hand 3, Pen, Cartoon and Pixel Art PNG sprite families come verbatim from the pinned upstream commit above. This includes the 720p, 1080p and 1440p landscape/portrait files under upstream `images/hand*.png`. Their upstream `tipX`/`tipY` placement metadata is preserved in `src/hand_assets.rs`.

The source repository does not rewrite or approximate those release sprites. `build.rs` retrieves them from immutable raw URLs containing the pinned commit, validates the PNG payloads and stores them in a persistent Cargo cache keyed by that commit. The Windows CI workflow caches the same directory between runs. CI/release builds fail if a required pinned sprite cannot be obtained, preventing release artifacts from silently substituting a fallback image.

For offline source builds, a pre-populated asset directory can be supplied with `INKPLAINER_HAND_ASSET_DIR`. Local builds without either the cache or network may use a transparent fallback so development can continue; that fallback is deliberately forbidden in CI/release builds.

All Rust source implementations in this repository are new native implementations unless a file or section explicitly states otherwise. The reused upstream hand assets remain covered by the upstream Apache-2.0 license and attribution.
