# PLAN 1: Alembic-RS Comprehensive Improvement Plan

**Created:** 2026-01-26
**Scope:** Performance fixes, API parity, viewer quality, path tracing research

---

## Phase 1: Viewer Performance Investigation & Fix

### 1.1 Profile Current Renderer
- [x] Add timing instrumentation to render() method (tracing spans already present)
- [x] Measure each pass: shadow, gbuffer, SSAO, lighting, transparent
- [x] Check ensure_depth_texture / ensure_gbuffer for unnecessary recreations (OK - size-based)
- [x] Profile compute_scene_hash cost (only on frame change, acceptable)
- [x] Check for GPU sync stalls (queue.submit blocking - normal)

### 1.2 Fix Performance Stuttering
- [x] **ROOT CAUSE FOUND**: 4 bind group allocations per frame (ssao, blur_h, blur_v, lighting)
- [x] **FIX APPLIED**: Cached bind groups, rebuilt only on texture resize / env map change
- [x] Replaced per-frame create_bind_group with persistent fields + dirty flag
- [x] Eliminated 2x write_buffer for blur direction params (now static H/V buffers)
- [x] Check if settings.save() is writing to disk on every frame (No - only on changed())
- [x] Look for Vec allocations in hot path (render loop) - minor, not the cause
- [x] Verify worker frame drain doesn't lose frames (OK - epoch-based)

### 1.3 Render Quality Improvements
- [x] Review StandardSurface shader implementation (correct GGX+Smith+Oren-Nayar)
- [x] Check shadow map quality (2048px, PCF 3x3, bias 0.002 - adequate)
- [x] Review SSAO quality parameters (configurable strength/radius, blur H+V)
- [x] **FIXED: IBL diffuse sampling** - was sampling 6 axis directions; now samples hemisphere around normal with weighted tangent/bitangent/sky for much better irradiance approximation

---

## Phase 2: Core API Parity Audit

### 2.1 Missing Features from FINDINGS.md
- [x] `getChildHeader(index)` - already implemented
- [x] Instance API surface exists (isInstanceRoot, etc.) - stubs only
- [ ] Implement ogawa reader instance parsing (low priority - rare in practice)
- [ ] Review ReadArraySampleCache feasibility (low priority)

### 2.2 Binary Compatibility Verification
- [x] Run hash comparison tests on reference .abc files (test_bmw_binary_comparison passes)
- [x] Roundtrip tests pass (22 total tests)
- [ ] Test with C++ Alembic library reading our output files
- [x] Time sampling encoding verified via roundtrip tests
- [x] **Full writer audit**: All 8 schema writers verified vs C++ ref (3 bugs fixed)
- [x] **Full reader audit**: All readers verified vs C++ ref (3 bugs fixed)
- [x] curveBasisAndType: fixed from 3 string scalars to 1 uint8x4 scalar
- [x] nVertices: removed incorrect dot prefix
- [x] SubD schema version: v2→v1
- [x] OCurves width: .widths→width
- [x] ICurves knots/orders: added dot prefixes
- [x] IPoints velocities/widths: fixed property names

### 2.3 Python API Parity
- [x] ISampleSelector with index/time/floor/ceil/near modes
- [x] All 9 schema getValue() accept ISampleSelector
- [x] IObject.getMetaData() returns dict
- [x] IObject.getArchive() back-reference
- [ ] Typed IGeomParam classes (low priority — generic IGeomParam covers most use cases)
- [ ] OSchema write classes (low priority — direct write API covers all schemas)

---

## Phase 3: Test Coverage Enhancement

### 3.1 Core Tests
- [x] Edge cases for time sampling (cyclic, acyclic, uniform)
- [ ] Property types (all POD types, extents)
- [ ] Large file handling (>1GB)
- [ ] Concurrent read access (thread safety)

### 3.2 Geometry Schema Tests
- [x] PolyMesh: triangle, cube, animated, empty
- [x] SubD: quad with catmull-clark scheme
- [x] NuPatch: 3x3 bilinear roundtrip
- [x] Curves: all basis types, linear/cubic, multi-sample animated
- [x] Points: positions + ids roundtrip
- [x] Camera: default + custom parameters
- [x] Light: roundtrip
- [x] FaceSet: face indices roundtrip
- [x] Xform: translation matrix
- [x] SubD with crease/corner/holes edge cases
- [ ] NuPatch with trim curves
- [ ] Points with varying widths
- [x] Curves with NURBS knots/orders/widths

### 3.3 Write/Roundtrip Tests
- [x] Write all geometry types and verify roundtrip (30 write_tests)
- [x] BMW roundtrip + binary comparison
- [x] Deduplication test
- [x] Visibility roundtrip
- [x] Archive metadata roundtrip
- [ ] Roundtrip hash comparison with C++ library output

---

## Phase 4: Viewer Architecture & Path Tracing

### 4.1 Research: Cross-Platform Path Tracing Options
- [ ] Evaluate wgpu compute shader approach
- [ ] Evaluate embree-rs (CPU) option
- [ ] Research wgpu ray-tracing extension status (2026)
- [ ] Check if ash + VK_KHR_ray_tracing is viable for Win/Linux
- [ ] Evaluate Metal Performance Shaders for macOS

### 4.2 Design Decision: Path Tracing Architecture

**CHOSEN: Option 2 - Hybrid raster + compute shader path tracer**

Rationale:
- wgpu compute shaders work on ALL platforms (Vulkan, D3D12, Metal, WebGPU)
- No special HW features needed (no VK_KHR_ray_tracing)
- Rasterizer provides instant interactive feedback
- Path tracer refines progressively when camera is stationary
- Can optionally use wgpu ray query extension later as acceleration

### 4.3 Implementation Plan

#### Phase 4.3.1: BVH Infrastructure
- [ ] Create `src/viewer/pathtracer/` module
- [ ] Implement AABB and BVH node structures (GPU-friendly flat layout)
- [ ] Build SAH-based BVH on CPU from scene triangles
- [ ] Serialize BVH + triangles to GPU storage buffers
- [ ] Write BVH traversal in WGSL compute shader
- [ ] Test: render depth buffer via compute shader to validate BVH

#### Phase 4.3.2: Basic Path Tracer
- [ ] Ray generation from camera (compute shader)
- [ ] Lambertian diffuse + simple specular BRDF
- [ ] Direct lighting (sample light directions)
- [ ] Environment map sampling (equirectangular HDR)
- [ ] Progressive accumulation buffer (average over N frames)
- [ ] Reset accumulation on camera move
- [ ] Tone mapping pass (ACES filmic)

#### Phase 4.3.3: Standard Surface BSDF
- [ ] Port StandardSurface BSDF to path tracing context
- [ ] GGX microfacet importance sampling
- [ ] Metalness workflow (blend diffuse/specular)
- [ ] Coat layer with separate roughness
- [ ] Emission support

#### Phase 4.3.4: Advanced Features
- [ ] Transparency (refraction, thin-walled)
- [ ] Environment importance sampling (pre-computed CDF)
- [ ] Multiple Importance Sampling (MIS)
- [ ] Russian roulette for path termination
- [ ] Denoising pass (bilateral/temporal)

#### Phase 4.3.5: Integration
- [ ] Toggle between raster and path trace modes
- [ ] Auto-switch: raster during interaction, PT when idle
- [ ] Progressive quality indicator in status bar
- [ ] Settings: max samples, bounces, enable/disable PT

---

## Phase 5: Code Quality & Optimization

### 5.1 Rust-specific Optimizations
- [x] Zero-copy bytemuck try_cast_vec for f32/i32 array reads
- [x] #[inline] on hot-path functions (num_samples_from_property, compute_bounds_vec3)
- [x] String parsing: str::from_utf8 on slice instead of String::from_utf8 with to_vec()
- [ ] Review error handling (avoid panic in library code)
- [ ] Check thread safety guarantees (Send/Sync bounds)

### 5.2 Crate Quality
- [ ] Review murmur3 crate for edge cases
- [ ] Review spooky-hash crate for edge cases
- [ ] Review standard-surface shader correctness vs MaterialX spec

---

## Current Status

**Active Phase:** Phase 2.3 (Python API), Phase 3 (remaining tests), Phase 5 (Code Quality)
**Completed:** Phase 1 (Performance), Phase 2.1-2.2 (API Parity + Binary Compat), Phase 5.1 (Rust optimizations)
**Total tests:** 162 passing, 0 warnings
