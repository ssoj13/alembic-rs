# Alembic-RS - Remaining Tasks

## Status from Previous Plans

### plan2.md - Transform Bug
- [x] **FIXED** - Matrix multiplication order in `xform.rs:136` (`result = result * m`)

### plan.md - Viewer MVP (Phase 1)
- [x] Window with egui
- [x] Load .abc file (file dialog)
- [x] Extract PolyMesh vertices/indices
- [x] Render wireframe
- [x] Orbit camera (drag), zoom (scroll), pan (MMB)

---

## Remaining Tasks

### Rendering
- [x] Flat shading (dpdx/dpdy face normals in shader)
- [x] Smooth shading with normals (default mode)
- [x] Basic lighting (3-point rig + IBL already implemented)

### UI (Phase 2)
- [x] Object hierarchy tree (left panel with collapsible nodes)
- [x] Properties panel (shows selected object info in right panel)
- [x] Flat/smooth shading toggle (checkbox in Display settings)

### Animation (Phase 3)
- [x] Timeline slider (scrubbing) - already implemented
- [x] Play/pause/speed controls (FPS: 12/24/30/60) - already implemented
- [x] Frame interpolation - N/A (Alembic uses discrete samples)

---

## Library Core (from recent work)

### Completed
- [x] time_sampling_index() for all geom types (IPolyMesh, ISubD, ICamera, IXform, ICurves, IPoints, INuPatch, IFaceSet, ILight)
- [x] Python API schemas updated to use time_sampling_index()
- [x] All clippy warnings fixed

### Potential Improvements
- [ ] Python API: more schema coverage
- [ ] Documentation / examples
- [ ] Tests for time sampling
- [ ] Benchmark suite
