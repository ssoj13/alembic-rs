# Alembic Viewer - Plan

## Goal
Cross-platform 3D viewer for Alembic (.abc) files.

## Decision: egui + wgpu

**Stack:**
```
egui (UI) + eframe (windowing) + wgpu (GPU) + glam (math)
```

**Cross-platform support:**
| Platform | wgpu backend |
|----------|--------------|
| Windows  | Vulkan / DX12 |
| Linux    | Vulkan |
| macOS    | Metal |

**Why this stack:**
- Lightweight (~10 crates vs 200+ for Bevy)
- Fast compile (~30s incremental)
- Cross-platform from single codebase
- Full control over rendering
- `glam` already in project

---

## Features

### Rendering
- [x] Wireframe mode
- [ ] Flat shading
- [ ] Smooth shading with normals
- [ ] Basic lighting (directional + ambient)

### Animation
- [ ] Timeline scrubbing (slider)
- [ ] Play/pause/speed controls

### UI
- [ ] 3D viewport with orbit camera
- [ ] File open dialog
- [ ] Object hierarchy tree
- [ ] Properties panel

---

## Architecture

```
viewer/
  Cargo.toml
  src/
    main.rs        # eframe app entry
    app.rs         # App state, event loop
    camera.rs      # Orbit camera (arcball)
    render/
      mod.rs
      pipeline.rs  # wgpu render pipeline
      mesh.rs      # GPU mesh buffers
      shaders.wgsl # Vertex/fragment shaders
    ui/
      mod.rs
      viewport.rs  # 3D viewport (egui::PaintCallback)
      hierarchy.rs # Object tree
      timeline.rs  # Animation controls
```

---

## Dependencies

```toml
[package]
name = "alembic-viewer"
version = "0.1.0"
edition = "2021"

[dependencies]
eframe = "0.31"           # egui + winit + wgpu
egui = "0.31"
glam = "0.29"             # already in project
rfd = "0.15"              # native file dialog
alembic = { path = ".." }

# wgpu comes via eframe, but we need direct access for custom rendering
wgpu = "24"
bytemuck = { version = "1.21", features = ["derive"] }
```

---

## Phases

### Phase 1: MVP
- [ ] Window with egui
- [ ] Load .abc file (file dialog)
- [ ] Extract PolyMesh vertices/indices
- [ ] Render wireframe
- [ ] Orbit camera (drag), zoom (scroll), pan (MMB)

### Phase 2: UI
- [ ] Object hierarchy tree
- [ ] Properties panel
- [ ] Flat/smooth shading toggle

### Phase 3: Animation
- [ ] Timeline slider
- [ ] Playback controls
- [ ] Frame interpolation

---

## Next Steps

1. Create `viewer/` crate
2. Basic eframe window
3. wgpu pipeline for lines
4. Load mesh from .abc
5. Camera controls
