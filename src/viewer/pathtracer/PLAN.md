# Path Tracer Enhancement Plan

## Overview
Enhance the GPU path tracer to support full Autodesk Standard Surface materials,
HDR environment lighting, and improved sampling techniques.

## Current State
- [x] SAH-based BVH construction (CPU)
- [x] Basic ray-triangle intersection
- [x] Stack-based BVH traversal
- [x] Progressive accumulation
- [x] GGX specular + Lambertian diffuse
- [x] Metallic workflow with Fresnel
- [x] Russian roulette termination
- [x] Procedural sky gradient

## Phase 1: Full Standard Surface Material Support
**Goal**: Match the rasterizer's material capabilities

### 1.1 Expand GpuMaterial struct
- [x] Add transmission parameters (color, weight)
- [x] Add subsurface parameters (color, weight)
- [x] Add coat parameters (color, weight, roughness, IOR)
- [x] Add specular anisotropy
- [x] Add diffuse roughness (Oren-Nayar)
- [x] Update `material_from_params()` to copy all fields

### 1.2 Update WGSL Material struct
- [x] Match GpuMaterial layout in shader
- [x] Unpack all material fields in shading code

### 1.3 Implement coat layer
- [x] Add clearcoat GGX lobe
- [x] Layer coat on top of base material
- [x] Energy conservation between coat and base

### 1.4 Implement transmission/refraction
- [x] Add Snell's law refraction
- [x] Handle total internal reflection (TIR)
- [x] Fresnel-weighted blend between reflection/refraction
- [ ] Support colored transmission (Beer's law absorption) - TODO later

### 1.5 Implement subsurface scattering (approximation)
- [ ] Add diffusion profile approximation
- [ ] Blend with diffuse based on weight

## Phase 2: HDR Environment Lighting
**Goal**: Use loaded HDR maps for realistic lighting

### 2.1 Add environment texture binding
- [x] Create compute bind group for env texture + sampler
- [x] Pass env uniform (intensity, rotation, enabled)
- [x] Share texture from EnvironmentMap

### 2.2 Implement equirectangular sampling
- [x] Add `dir_to_equirect_uv()` function
- [x] Sample HDR on ray miss
- [x] Apply intensity and rotation

### 2.3 Importance sample environment (optional)
- [ ] Build CDF for environment importance sampling
- [ ] Sample bright areas preferentially

## Phase 3: Next Event Estimation (NEE)
**Goal**: Reduce noise with direct light sampling

### 3.1 Add sun/directional light sampling
- [x] Define sun direction and intensity (constants for now)
- [x] Shadow ray toward sun on each hit
- [x] Sample sun disc for soft shadows
- [x] Evaluate diffuse + specular BRDF for sun
- [ ] MIS weight with BSDF sampling (skipped for simplicity)

### 3.2 Add environment light NEE
- [ ] Sample random direction on hemisphere
- [ ] Weight by environment luminance
- [ ] Combine with BSDF via MIS

## Phase 4: Advanced Features

### 4.1 Texture support
- [ ] Add UV coordinates to triangles
- [ ] Bind texture atlases
- [ ] Sample albedo/roughness/normal maps

### 4.2 Denoising
- [ ] Output auxiliary buffers (albedo, normal, depth)
- [ ] Integrate temporal accumulation with motion vectors
- [ ] (Optional) OIDN/OptiX denoiser

### 4.3 Performance optimizations
- [ ] Wavefront path tracing
- [ ] Persistent threads
- [ ] Improved BVH quality (SBVH)

---

## Implementation Order

1. **Phase 1.1-1.2**: Expand material struct (Rust + WGSL)
2. **Phase 2.1-2.2**: HDR environment sampling
3. **Phase 1.4**: Transmission/refraction
4. **Phase 1.3**: Coat layer
5. **Phase 3.1**: Sun/directional NEE
6. **Phase 1.5**: Subsurface approximation
7. **Phase 3.2**: Environment NEE
8. **Phase 4**: Advanced features

---

## Progress Log

### Session 1 (Current)
- [x] Phase 1.1: Expanded GpuMaterial to 144 bytes (9 vec4s) matching StandardSurfaceParams
- [x] Phase 1.2: Updated WGSL Material struct with all Standard Surface fields
- [x] Phase 1.3: Implemented coat layer (clearcoat GGX)
- [x] Phase 1.4: Implemented transmission/refraction (Snell's law, TIR)
- [x] Phase 2.1: Added environment texture binding (6,7,8)
- [x] Phase 2.2: Implemented equirectangular HDR sampling in shader
- [x] Fixed pt_max_bounces not being passed to shader (was hardcoded)
- [x] Reorganized UI into collapsible sections:
  - Render Mode (Rasterizer / Path Tracer toggle)
  - Rasterizer settings (hidden when PT active)
  - SSAO settings (hidden when PT active)
  - Path Tracer settings (hidden when rasterizer active)
  - Environment
  - Lighting
  - Display (AA, background color)
  - Debug (test cube, clear scene)
- [x] Phase 3.1: Added Next Event Estimation for sun light
  - Shadow ray tracing function
  - Sun disc sampling for soft shadows
  - Direct diffuse + specular contribution
