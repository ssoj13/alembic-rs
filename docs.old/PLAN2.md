# Alembic-RS Transform Bug Analysis Report

## Problem Description

ABC files from `data/*.abc` (e.g., `flo.abc`) load incorrectly in the viewer - geometry parts are scattered instead of forming a compact structure. This indicates a transform calculation bug.

## Root Cause Analysis

### Matrix Convention Mismatch

**Imath (C++ Alembic reference)**:
- **Row-major** storage: `x[row][col]`
- **Row-vectors**: `v' = v * M` (vector multiplied from left)
- Translation stored in **row 3**: `x[3][0], x[3][1], x[3][2]`

**glam (Rust)**:
- **Column-major** storage: `col[col][row]`  
- **Column-vectors**: `v' = M * v` (vector multiplied from right)
- Translation stored in **column 3**

### The Bug Location

**File**: `src/geom/xform.rs`, line 136

**Current (WRONG)**:
```rust
// Alembic uses left-multiply: ret = m * ret (see XformSample.cpp:518)
result = m * result;
```

**Should be (CORRECT)**:
```rust
// For glam column-vectors, equivalent of Imath's left-multiply is right-multiply
result = result * m;
```

### Mathematical Proof

Given operations `[op0, op1, op2]`:

**In Imath (row-vectors)**:
```
Loop iteration:
- i=0: ret = m0 * I = m0
- i=1: ret = m1 * m0
- i=2: ret = m2 * m1 * m0

Point transform: v' = v * ret = v * (m2 * m1 * m0)
```

**In glam with current code** (`result = m * result`):
```
Same accumulation: result = m2 * m1 * m0
Point transform: v' = result * v = (m2 * m1 * m0) * v
```

**These produce DIFFERENT results!**

Example with `ops = [Translate(1,0,0), Scale(2,2,2)]`:
- Imath: point (0,0,0) -> result (1,0,0)
- glam (current): point (0,0,0) -> result (2,0,0) **WRONG!**

**In glam with fix** (`result = result * m`):
```
Loop iteration:
- i=0: result = I * m0 = m0
- i=1: result = m0 * m1
- i=2: result = m0 * m1 * m2

Point transform: v' = result * v = (m0 * m1 * m2) * v
```

This is equivalent to Imath's `v * (m2 * m1 * m0)` because glam creates transposed matrices for each operation.

## Files Analyzed

| File | Status | Notes |
|------|--------|-------|
| `src/geom/xform.rs:136` | **BUG** | Matrix multiplication order wrong |
| `src/viewer/mesh_converter.rs:171` | OK | `parent * local` correct for hierarchy |
| `src/ogawa/writer.rs:1543` | OK | Proper transpose for writing |
| `src/viewer/renderer.rs` | OK | Standard glam usage |
| `src/viewer/camera.rs` | OK | Standard glam usage |

## The Fix

### xform.rs line 136

**Before**:
```rust
result = m * result;
```

**After**:
```rust
result = result * m;
```

### Why mesh_converter.rs is Correct

```rust
let world_transform = parent_transform * local_transform;
```

For hierarchy traversal (root -> parent -> child):
- glam: `world = parent * local` means `v' = parent * (local * v)`
- This applies local first, then parent - **CORRECT**

## Verification Steps

1. Apply the fix to `xform.rs`
2. Rebuild: `cargo build --release`
3. Test with `data/flo.abc` in viewer
4. Compare with reference Alembic C++ output using `alembic.exe`

## Additional Notes

### Matrix Operation Reading (xform.rs:123-133)

The `XformOpType::Matrix` case correctly transposes from Alembic row-major to glam column-major:
```rust
glam::Mat4::from_cols(
    glam::vec4(v[0], v[4], v[8], v[12]),   // col 0
    glam::vec4(v[1], v[5], v[9], v[13]),   // col 1
    glam::vec4(v[2], v[6], v[10], v[14]),  // col 2
    glam::vec4(v[3], v[7], v[11], v[15]),  // col 3
)
```

### Individual Operations (Scale, Translate, Rotate)

These use glam's native constructors (`from_scale`, `from_translation`, `from_rotation_x/y/z`, `from_axis_angle`) which already create column-vector matrices - **no transposition needed**.

## Summary

**Single bug**: Wrong matrix multiplication order in `xform.rs:136`

**Fix**: Change `result = m * result` to `result = result * m`

**Impact**: This will fix all scattered geometry issues when loading ABC files.
