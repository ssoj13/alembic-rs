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
- [ ] Flat shading
- [ ] Smooth shading with normals
- [ ] Basic lighting (directional + ambient)

### UI (Phase 2)
- [ ] Object hierarchy tree
- [ ] Properties panel
- [ ] Flat/smooth shading toggle

### Animation (Phase 3)
- [ ] Timeline slider (scrubbing)
- [ ] Play/pause/speed controls
- [ ] Frame interpolation

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
