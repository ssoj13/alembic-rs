# Viewport Rendering Bug Fix Plan

## Executive Summary

**Root Cause**: Fullscreen shader UV coordinates are inverted, causing 180-degree image rotation in GBuffer render path.

**Symptom**: 
- `opacity < 1.0` (xray_alpha < 1.0) → Works correctly (transparent path)
- `opacity == 1.0` (xray_alpha == 1.0) → Broken (GBuffer path with flipped passes)

**Impact**: All fullscreen post-processing passes (SSAO, SSAO Blur, Lighting) produce rotated output.

---

## Bug Analysis

### Location
**File**: `src/viewer/renderer/shaders.rs`
**Lines**: 21, 110, 157 (three fullscreen vertex shaders)

### Current WRONG Code
```wgsl
out.uv = vec2<f32>(1.0 - (pos.x * 0.5 + 0.5), pos.y * 0.5 + 0.5);
```

### Correct Code
```wgsl
out.uv = vec2<f32>(pos.x * 0.5 + 0.5, 0.5 - pos.y * 0.5);
```

### Math Explanation

| Screen Position | Current UV (Wrong) | Correct UV |
|----------------|-------------------|------------|
| Top-Left (-1,+1) | (1.0, 1.0) | (0.0, 0.0) |
| Top-Right (+1,+1) | (0.0, 1.0) | (1.0, 0.0) |
| Bottom-Left (-1,-1) | (1.0, 0.0) | (0.0, 1.0) |
| Bottom-Right (+1,-1) | (0.0, 0.0) | (1.0, 1.0) |

The current formula flips both X and Y, resulting in 180° rotation.

---

## Fix Plan

### Phase 1: Fix UV Coordinates (Critical)

- [x] **1.1** Fix SSAO_SHADER vs_fullscreen (line 21)
  ```
  File: src/viewer/renderer/shaders.rs
  Line: 21
  Change: out.uv = vec2<f32>(1.0 - (pos.x * 0.5 + 0.5), pos.y * 0.5 + 0.5);
  To:     out.uv = vec2<f32>(pos.x * 0.5 + 0.5, 0.5 - pos.y * 0.5);
  ```

- [x] **1.2** Fix SSAO_BLUR_SHADER vs_fullscreen (line 110)
  ```
  File: src/viewer/renderer/shaders.rs
  Line: 110
  Same change as above
  ```

- [x] **1.3** Fix LIGHTING_SHADER vs_fullscreen (line 157)
  ```
  File: src/viewer/renderer/shaders.rs
  Line: 157
  Same change as above
  ```

### Phase 2: Code Cleanup (Optional but Recommended)

- [x] **2.1** Remove unused `xray_active` variable
  ```
  File: src/viewer/renderer/mod.rs
  Line: 1328
  Remove: let xray_active = false;
  ```

- [ ] **2.2** Consider making `use_gbuffer` configurable or remove hardcoding
  ```
  File: src/viewer/renderer/mod.rs
  Line: 1309
  Currently: let use_gbuffer = true;
  Consider: Make this a setting or remove the flag entirely
  ```

- [ ] **2.3** Audit #[allow(dead_code)] annotations
  - Review each dead_code annotation
  - Remove truly dead code
  - Document intentionally kept code

### Phase 3: Streamline Render Paths (Future)

- [ ] **3.1** Consider unifying UV coordinate helper
  ```rust
  // Create a shared constant or function for NDC-to-UV conversion
  fn ndc_to_uv(ndc: vec2<f32>) -> vec2<f32> {
      return vec2<f32>(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5);
  }
  ```

- [ ] **3.2** Review GBuffer path necessity
  - Currently always enabled (`use_gbuffer = true`)
  - Consider when GBuffer path provides benefits vs direct rendering
  - SSAO only makes sense with GBuffer
  - Without SSAO, direct rendering might be simpler

---

## Testing

### Manual Test Cases

1. **Opacity 1.0 Test**
   - Load any mesh
   - Ensure xray_alpha slider is at 1.0
   - Verify mesh renders correctly (not flipped/rotated)

2. **Opacity < 1.0 Test**
   - Load any mesh
   - Set xray_alpha slider to 0.5
   - Verify mesh renders correctly (should work before and after fix)

3. **SSAO Test**
   - Enable SSAO checkbox
   - Verify occlusion renders correctly (not inverted)

4. **HDR Background Test**
   - Load HDR environment
   - Verify background renders correctly

5. **Transparent + Opaque Mix**
   - Load scene with multiple meshes
   - Set some meshes to transparent via xray_alpha
   - Verify both opaque and transparent meshes render correctly

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| UV fix breaks other features | Low | High | Test all render modes |
| HDR sampling affected | Medium | Medium | Test HDR backgrounds |
| Shadow mapping affected | Low | Low | Shadows use different UV calc |

---

## Rollback Plan

If issues arise, revert shaders.rs changes:
```bash
git checkout HEAD~1 -- src/viewer/renderer/shaders.rs
```

---

## Approval Required

This plan requires approval before implementation.

**Changes Summary**:
1. Fix 3 UV coordinate calculations in shaders.rs
2. Optional: Remove dead code
3. Optional: Streamline render path logic

**Estimated Lines Changed**: ~10 lines (critical fix only)

---

## References

- `src/viewer/AGENTS.md` - Architecture documentation
- `src/viewer/DIAGRAMS.md` - Mermaid diagrams
- wgpu coordinate conventions: https://wgpu.rs
