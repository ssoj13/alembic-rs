# Eevee-lite Plan

## Goals
- Stable forward renderer first (no postfx), then add G-Buffer + SSAO, then transparency + postfx.
- One consistent render path: clear stages, deterministic depth, no hidden flips.
- Keep UI toggles accurate and persistent.

---

## Stage 0: Stabilize (Baseline Forward Pass)
**Objective:** get a correct, solid view with depth and materials, zero post-processing.

1) **Single forward pass (opaque)**
   - Render directly into viewport target (no offscreen, no composite).
   - Depth enabled, depth write enabled, compare Less.
   - No blending.
   - Files: `src/viewer/renderer/mod.rs`, `src/viewer/renderer/passes.rs`, `src/viewer/renderer/pipelines.rs`.

2) **Wireframe pass separated**
   - Wireframe uses its own pipeline, still depth-tested.
   - Toggle: `show_wireframe`.

3) **Transparent pass (placeholder)**
   - Disabled by default. Keep the path but no objects flagged yet.
   - Ensures pipeline separation is ready.

4) **Camera + Depth sanity**
   - Maintain stable near/far based on bounds.
   - No Y-flips anywhere except where strictly needed.

Deliverable: solid mesh rendering with correct depth; no postfx; no X-ray.

---

## Stage 1: G-Buffer (Eevee-lite core)
**Objective:** render albedo/normal/roughness/metal + depth to G-Buffer.

1) **G-Buffer pass**
   - Write: albedo+roughness, normal+metal, occlusion (white), depth.
   - Separate render pass with explicit color formats.

2) **Lighting pass (fullscreen)**
   - Reconstruct lighting from G-Buffer (simple PBR + IBL).
   - Output color into offscreen target.

3) **Composite**
   - Blit final color to swapchain.

Deliverable: same look as Stage 0, but computed with G-Buffer.

---

## Stage 2: SSAO
**Objective:** add SSAO as a clean post-pass using G-Buffer depth + normals.

1) **SSAO pass**
   - Depth/normal samples, no flips.
   - Occlusion written to a single-channel texture.

2) **Composite**
   - Multiply occlusion into lighting output.

Deliverable: toggleable SSAO that does not flip or misalign.

---

## Stage 3: Transparency
**Objective:** implement proper transparency (Eevee-like order).

1) **Classification**
   - Opaque vs Transparent based on material opacity.

2) **Transparent pass**
   - Depth test, depth write off, alpha blending.
   - Sorted back-to-front per frame.

3) **Double-sided**
   - Per-pass cull toggle.

Deliverable: stable transparency without breaking opaque depth.

---

## Stage 4: Post FX
**Objective:** add optional tonemap/bloom.

1) **Tone mapping**
   - Filmic curve, exposure control.

2) **Bloom**
   - Threshold + blur + composite.

Deliverable: Eevee-like look without pipeline breakage.

---

## Stage 5: Optional
- Screen-space reflections (SSR).
- Shadow improvements (PCSS/EVSM).
- Temporal AA (TAA) if needed.

---

## UI / Settings
- Every toggle maps 1:1 to a renderer flag.
- Persistent state in `Settings`.

---

## Execution Order (Do Not Skip)
1) Stage 0 baseline forward.
2) Stage 1 G-Buffer.
3) Stage 2 SSAO.
4) Stage 3 Transparency.
5) Stage 4 Post FX.

---

## References
- Blender Eevee docs (pipeline overview)
- LearnOpenGL deferred shading
- Frostbite PBR notes
