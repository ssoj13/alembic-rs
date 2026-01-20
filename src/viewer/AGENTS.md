# Viewer Render Pipeline Analysis

## Architecture Overview

The viewer has **two distinct render paths** based on mesh opacity:

### 1. GBuffer Path (opacity >= 0.999, xray_alpha == 1.0)
```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          GBuffer Render Path                                │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐                   │
│  │ Shadow Pass  │───>│ GBuffer Pass │───>│  SSAO Pass   │                   │
│  │ (depth only) │    │ (3 targets)  │    │ (fullscreen) │                   │
│  └──────────────┘    └──────────────┘    └──────────────┘                   │
│         │                   │                   │                           │
│         v                   v                   v                           │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐                   │
│  │ Shadow Map   │    │ Albedo+Rough │    │ Occlusion    │                   │
│  │ Depth32Float │    │ Normal+Metal │    │ R8Unorm      │                   │
│  └──────────────┘    │ Occlusion    │    └──────────────┘                   │
│                      └──────────────┘           │                           │
│                             │                   │                           │
│                             └────────┬──────────┘                           │
│                                      v                                      │
│                             ┌──────────────┐    ┌──────────────┐            │
│                             │SSAO Blur x2  │───>│Lighting Pass │            │
│                             │ (fullscreen) │    │ (fullscreen) │            │
│                             └──────────────┘    └──────────────┘            │
│                                                        │                    │
│                                                        v                    │
│                                                 ┌──────────────┐            │
│                                                 │ Final Output │            │
│                                                 │ (to egui)    │            │
│                                                 └──────────────┘            │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 2. Transparent Path (opacity < 0.999 OR xray_alpha < 1.0)
```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        Transparent Render Path                              │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐                   │
│  │ Shadow Pass  │───>│ GBuffer Pass │───>│Lighting Pass │                   │
│  │ (opaque)     │    │ (opaque)     │    │ (compose)    │                   │
│  └──────────────┘    └──────────────┘    └──────────────┘                   │
│                                                 │                           │
│                                                 v                           │
│                                          ┌──────────────┐                   │
│                                          │Transparent   │                   │
│                                          │Pass (sorted) │                   │
│                                          │fs_main shader│                   │
│                                          └──────────────┘                   │
│                                                 │                           │
│                                                 v                           │
│                                          ┌──────────────┐                   │
│                                          │ Final Output │                   │
│                                          └──────────────┘                   │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Key Files

| File | Purpose |
|------|---------|
| `renderer/mod.rs` | Main Renderer struct, render() orchestration |
| `renderer/passes.rs` | Individual render pass implementations |
| `renderer/postfx.rs` | Post-processing pipeline setup (SSAO, lighting) |
| `renderer/shaders.rs` | Fullscreen WGSL shaders (SSAO, blur, lighting) |
| `renderer/pipelines.rs` | Scene rendering pipelines |
| `viewport.rs` | egui integration, render texture management |
| `settings.rs` | Persistent settings (xray_alpha, etc.) |

## Data Flow

```
User Input (xray_alpha slider)
        │
        v
┌───────────────────┐
│ ViewerApp         │
│ settings.xray_alpha│
└─────────┬─────────┘
          │
          v
┌───────────────────┐
│ Renderer          │
│ self.xray_alpha   │
└─────────┬─────────┘
          │
          v
┌───────────────────────────────────────┐
│ render() - Decision Point             │
│                                       │
│ effective_opacity = mesh.opacity *    │
│                     self.xray_alpha   │
│                                       │
│ if effective_opacity < 0.999:         │
│   → transparent_meshes (sorted)       │
│ else:                                 │
│   → opaque_mesh_names                 │
└───────────────────────────────────────┘
          │
          ├──────────────────────────────────────┐
          │                                      │
          v                                      v
┌─────────────────────┐              ┌─────────────────────┐
│ GBuffer Path        │              │ Transparent Path    │
│ (BROKEN UV!)        │              │ (WORKS CORRECTLY)   │
│                     │              │                     │
│ Uses fullscreen     │              │ Uses fs_main        │
│ shaders with        │              │ standard_surface    │
│ WRONG UV coords     │              │ direct rendering    │
└─────────────────────┘              └─────────────────────┘
```

## Shader Pipeline

### Fullscreen Quad Generation (BROKEN)
```wgsl
// Current WRONG code in shaders.rs (lines 21, 110, 157):
var positions = array<vec2<f32>, 3>(
    vec2<f32>(-1.0, -1.0),
    vec2<f32>(3.0, -1.0),
    vec2<f32>(-1.0, 3.0)
);
let pos = positions[index];
out.pos = vec4<f32>(pos, 0.0, 1.0);
out.uv = vec2<f32>(1.0 - (pos.x * 0.5 + 0.5), pos.y * 0.5 + 0.5);
//       ^^^^^^^ WRONG! Flips X coordinate
```

### Expected UV Mapping
```
Screen Position (clip space)  →  Texture UV (wgpu convention)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Top-Left     (-1, +1)         →  (0, 0)
Top-Right    (+1, +1)         →  (1, 0)
Bottom-Left  (-1, -1)         →  (0, 1)
Bottom-Right (+1, -1)         →  (1, 1)
```

### Current BROKEN UV Mapping
```
Screen Position (clip space)  →  Current UV (WRONG!)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Top-Left     (-1, +1)         →  (1.0, 1.0) ← Should be (0, 0)!
Top-Right    (+1, +1)         →  (0.0, 1.0) ← Should be (1, 0)!
Bottom-Left  (-1, -1)         →  (1.0, 0.0) ← Should be (0, 1)!
Bottom-Right (+1, -1)         →  (0.0, 0.0) ← Should be (1, 1)!

Result: Image rotated 180 degrees (both X and Y flipped)!
```

## Additional Issues Found

1. **Unused variable**: `xray_active = false` at mod.rs:1328 - never used
2. **Hardcoded flag**: `use_gbuffer = true` at mod.rs:1309 - always true
3. **Many #[allow(dead_code)]** - indicates incomplete refactoring
4. **TODO**: viewport.rs:221 - "Calculate scene bounds" not implemented
