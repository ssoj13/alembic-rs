# TODO

## Viewer Improvements

### Lighting
- [x] Use collected `SceneLight` data for actual scene lighting
- [x] Toggle between default lights and scene lights (checkbox in Lighting panel)
- [ ] Support multiple light types (point, directional, spot) - currently uses first light only
- [ ] Light intensity and color from Alembic properties

### Materials
- [x] Apply metallic/roughness from shader params
- [x] Material inheritance flattening (inherits_path resolution)
- [ ] Support more shader targets beyond arnold/renderman
- [x] UI panel for material properties inspection

### Points Rendering
- [x] Store per-point widths from Alembic
- [ ] Point sprites or sphere impostors (requires instanced pipeline)

### Camera
- [x] Animate camera from Alembic camera samples
- [x] Camera switching UI (dropdown in View menu)

### UI
- [x] Properties inspector panel (right panel shows selected object details)
- [x] Timeline with keyframe markers (tick marks under slider)
- [x] Hierarchy tree view with visibility toggles
- [x] Hierarchy filter with wildcard support (e.g., wheel*)

## Parity (Complete)

- [x] Xform ops decoding (translate hints, matrix ops) - verified identical to C++ Alembic
- [x] Mesh cache key collision (fixed: use mesh_path instead of mesh_name)
- [x] ArchiveBounds edge cases (NaN/Inf validation, safe defaults)
- [x] Material flattening advanced inheritance

## Future Features

- [ ] Export to USD format
- [ ] Export to glTF format
- [ ] Multiple light support with type-specific rendering
- [ ] Instanced point sprites for large point clouds
