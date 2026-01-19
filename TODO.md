# Viewer Improvement Plan (Detailed)

## Phase 0 - Baseline Stabilization
- Verify current viewer renders `bed.abc` and `gears_out.abc` without disappearing meshes when `Double Sided` is on.
- Fix transparency handling for X-Ray (`xray_alpha < 1.0`) by disabling depth writes and sorting transparent meshes back-to-front.
- Keep a quick regression checklist: `bed.abc`, `gears.abc`, `gears_out.abc`.

## Phase 1 - Render Pass Separation (Streamline)
- Split render into explicit passes:
  - Shadow pass (depth-only).
  - Opaque pass (depth write on, cull depending on `double_sided`).
  - Transparent pass (depth write off, back-to-front sort).
  - Lines/points pass (after meshes).
- Centralize pipeline selection in a single helper to avoid divergent logic.
- Track last camera position in `Renderer` to enable transparent sorting.

## Phase 2 - Material System & Batching
- Introduce a stable `MaterialId` and a `MaterialRegistry` mapping id -> bind group + params.
- Split each mesh into `SubMesh` segments:
  - `index_range` + `material_id`.
  - Use Alembic face-sets to drive submesh creation.
- Batch draw calls by material to reduce bind group changes.
- Add a per-submesh render path to support multiple materials in one mesh.

## Phase 3 - Shading Quality
- Add tangent generation (MikkTSpace) for normal maps.
- Add IBL prefilter + BRDF LUT for better specular response.
- Optional: depth prepass for better transparency/alpha-tested geometry.
- Improve `Auto Normals` behavior to be consistent with double-sided lighting.

## Phase 4 - Debug & UX
- Debug toggles: show normals, show UVs, show face-set boundaries.
- Material inspection in UI (display scalar params and texture info).
- Capture render stats (draw calls, materials, transparent count).

## Current Focus
- Phase 1: explicit opaque/transparent passes; transparent meshes sorted back-to-front.
