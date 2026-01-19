# Plan 3 - Viewer Refactor and Quality Improvements

## Scope
Focus on viewer rendering correctness and architecture streamlining, with a path to multi-material support.

## Plan
1) Stabilize transparency and double-sided rendering
- Disable depth writes in transparent pipelines (X-Ray).
- Track camera position and sort transparent meshes back-to-front.
- Validate with `bed.abc` and `gears_out.abc`.

2) Split render into clear passes
- Shadow (depth-only) -> Opaque -> Transparent -> Lines/Points.
- Centralize pipeline selection.

3) Material registry and batching
- MaterialId and registry for reusing bind groups.
- Submesh ranges with per-material draw.
- Face-set integration for Alembic meshes.

4) Shading upgrades
- Tangent generation for normal maps.
- IBL prefilter + BRDF LUT.

5) Debugging and UX
- Normal/UV/face-set debug views.
- Render stats overlay.

## Status
- Step 1 in progress: transparent pipeline selection + sorting.
