# Alembic-rs TODO

## Viewer Parity Issues

Schemas implemented in library but NOT USED in viewer:

### Working
- [x] IPolyMesh - renders correctly
- [x] ICurves - renders correctly  
- [x] IXform - transforms work

### Missing in Viewer
- [ ] IMaterial - read material assignments, convert to StandardSurface colors
- [ ] ISubD - render as mesh (or with subdivision)
- [ ] IPoints - render as points/sprites
- [ ] ICamera - add "use scene camera" option
- [ ] ILight - basic scene lighting support

## Recent Optimizations (Done)
- [x] Mmap-only file reading (removed File fallback)
- [x] Rayon parallelization for mesh conversion (42% faster)
- [x] Arc zero-copy for vertex/index data
- [x] Mesh caching between frames
- [x] FPS combo with standard film/TV framerates
