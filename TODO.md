# TODO

## Viewer Improvements

### Lighting
- [ ] Use collected `SceneLight` data for actual scene lighting
- [ ] Support multiple light types (point, directional, spot)
- [ ] Light intensity and color from Alembic properties

### Materials
- [ ] Apply metallic/roughness from shader params (currently only base_color)
- [ ] Support more shader targets beyond arnold/renderman
- [ ] UI panel for material properties inspection

### Points Rendering
- [ ] Use per-point widths for variable point sizes
- [ ] Point sprites or sphere impostors for better quality

### Camera
- [ ] Animate camera from Alembic camera samples
- [ ] Camera switching UI (already have scene_cameras collected)

## Parity (~0.5% remaining)

- [x] Xform ops decoding (translate hints, matrix ops) - verified identical to C++ Alembic
- [x] Mesh cache key collision (fixed: use mesh_path instead of mesh_name)
- [ ] ArchiveBounds edge cases
- [ ] Material flattening advanced inheritance

## Future Features

- [ ] Export to USD format
- [ ] Export to glTF format
- [ ] Properties inspector panel in viewer
- [ ] Timeline with keyframe markers
- [x] Hierarchy tree view with visibility toggles
- [x] Hierarchy filter with wildcard support (e.g., wheel*)
