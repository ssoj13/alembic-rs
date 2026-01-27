# PLAN 2: Viewer Path Tracing Architecture

**Created:** 2026-01-26
**Scope:** Cross-platform path tracing for the alembic-rs viewer

---

## Research Summary

### Approach: WGSL Compute Shader Path Tracing

**Chosen** over embree-rs (CPU-only, unmaintained bindings), wgpu EXPERIMENTAL_RAY_QUERY
(not stable, no macOS), and rust-gpu (toolchain complexity).

**Why WGSL compute:**
- Works on ALL wgpu backends: Vulkan, D3D12, Metal, WebGPU
- No external dependencies beyond existing wgpu
- Multiple reference implementations exist (wgpu-path-tracing, weekend-raytracer-wgpu)
- Integrates with our existing rasterizer for hybrid mode
- Progressive refinement when camera idle, instant raster feedback when interacting

### Future Enhancement
- `EXPERIMENTAL_RAY_QUERY` as optional HW-accelerated fast-path when it stabilizes
- Design BVH data layout to be compatible with both software and HW acceleration structures

---

## Phase 1: BVH Infrastructure

### 1.1 Data Structures
- [x] Create `src/viewer/pathtracer/` module
- [x] `bvh.rs` - AABB node struct (GPU-friendly flat array layout)
- [x] `build.rs` - SAH-based BVH builder on CPU from scene triangles (4 unit tests)
- [x] `gpu_data.rs` - Serialize BVH + triangles + materials to GPU storage buffers

### 1.2 WGSL BVH Traversal
- [x] `bvh_traverse.wgsl` - Stack-based BVH traversal compute shader (Moller-Trumbore + slab AABB)
- [ ] Test: render depth buffer via compute to validate BVH correctness
- [ ] Benchmark: measure rays/sec on reference scene
- [x] Integration: compute.rs pipeline + blit.wgsl tone mapping + scene_convert.rs bridge

---

## Phase 2: Basic Path Tracer

### 2.1 Ray Generation
- [ ] Camera ray generation in compute shader (pinhole + thin lens DOF)
- [ ] Progressive accumulation buffer (average over N frames)
- [ ] Reset accumulation on camera move / scene change

### 2.2 Shading
- [ ] Lambertian diffuse BRDF
- [ ] GGX microfacet specular (matches existing rasterizer)
- [ ] Direct lighting (sample light directions)
- [ ] Environment map sampling (equirectangular HDR, reuse existing env map)

### 2.3 Integration
- [ ] Tone mapping pass (ACES filmic, reuse existing)
- [ ] Toggle raster / path trace modes
- [ ] Auto-switch: raster during interaction, PT when idle (>0.5s)
- [ ] Progressive quality indicator in status bar

---

## Phase 3: Standard Surface BSDF

- [ ] Port StandardSurface params from rasterizer to PT context
- [ ] GGX importance sampling for specular lobe
- [ ] Metalness workflow (blend diffuse/specular by metallic)
- [ ] Coat layer with separate roughness
- [ ] Emission support
- [ ] Opacity/transparency (thin-walled mode)

---

## Phase 4: Advanced Features

- [ ] Environment importance sampling (pre-computed CDF)
- [ ] Multiple Importance Sampling (MIS) - balance heuristic
- [ ] Russian roulette for path termination
- [ ] Next Event Estimation (NEE) for direct illumination
- [ ] Refraction (non-thin-walled transparency)
- [ ] Temporal denoising pass (bilateral filter)

---

## Architecture Notes

```
Scene Load --> BVH Build (CPU, rayon) --> GPU Buffers
                                              |
Camera Move --> Reset Accumulation            v
                                    Compute Shader (per-pixel):
                                      1. Generate ray
                                      2. BVH traverse -> hit
                                      3. Shade (Standard Surface BRDF)
                                      4. Bounce (up to N bounces)
                                      5. Accumulate to buffer
                                              |
                                              v
                                    Tone Map Pass --> Display
```

### GPU Buffer Layout
- `triangles: StorageBuffer<[Triangle]>` - packed vertex data (pos, normal, uv)
- `bvh_nodes: StorageBuffer<[BVHNode]>` - flat BVH (left/right/aabb/leaf_start/leaf_count)
- `materials: StorageBuffer<[Material]>` - Standard Surface params per material
- `accumulation: StorageTexture<rgba32float>` - progressive accumulation
- `frame_count: Uniform<u32>` - for averaging

### BVH Node Layout (GPU-friendly, 32 bytes)
```wgsl
struct BVHNode {
    aabb_min: vec3<f32>,    // 12 bytes
    left_or_first: u32,     // 4 bytes (if leaf: first triangle index)
    aabb_max: vec3<f32>,    // 12 bytes
    count: u32,             // 4 bytes (0 = internal node, >0 = leaf with N triangles)
};
```

---

## Status

**Current:** Phase 1 complete. Compute pipeline, blit shader, scene converter all ready. Phase 2 next (wire into Renderer, ray generation, accumulation).
**Dependencies:** None - uses existing wgpu infrastructure.
**Estimated scope:** ~2000-3000 lines for Phase 1+2 (functional path tracer).
