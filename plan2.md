# Bug Hunt Report - alembic-rs (2026-01-20)

## Executive Summary

This comprehensive bug hunt analyzed the entire alembic-rs codebase across 94+ source files. The analysis identified **25+ actionable issues** ranging from critical bugs to low-priority cleanup tasks.

**Key Findings:**
- 3 HIGH severity bugs requiring immediate attention
- 8 MEDIUM severity issues affecting functionality/performance
- 14+ LOW severity items (dead code, duplication, documentation)

---

## HIGH SEVERITY ISSUES

### 1. Viewer: Material Inheritance Resolution Order Bug
**File:** `src/viewer/mesh_converter.rs:588-598`
**Impact:** Materials with inherited values render incorrectly

Material properties are applied BEFORE `resolve_material_inheritance()` is called:
```rust
// Line 588-595: Material props applied here
for (full_name, mat) in &mat_props {
    // ... apply material
}

// Line 598: But inheritance resolved AFTER!
let mat_props = resolve_material_inheritance(&mat_props);
```

**Fix:** Move `resolve_material_inheritance()` call before the material application loop.

---

### 2. Viewer: Abrupt Exit Bypasses Cleanup
**File:** `src/viewer/app.rs:196`
**Impact:** Settings not saved, potential resource leaks

```rust
std::process::exit(0)  // Bypasses on_exit() callback
```

**Fix:** Use `ctx.send_viewport_cmd(egui::ViewportCommand::Close)` instead.

---

### 3. Python Bindings: Matrix Convention Mismatch
**File:** `src/python/write.rs:487-491`
**Impact:** Incorrect transforms when users pass row-major matrices

Comment says "row-major" but `from_cols_array_2d` expects column-major:
```rust
/// Add sample from 4x4 matrix (row-major, f32).  // <-- WRONG
fn addMatrixSample(&mut self, matrix: [[f32; 4]; 4], inherits: bool) -> PyResult<()> {
    let m = glam::Mat4::from_cols_array_2d(&matrix);  // expects column-major!
```

**Fix:** Either document as column-major OR transpose the input matrix.

---

## MEDIUM SEVERITY ISSUES

### 4. Viewer: Scene Cameras/Lights Never Refresh
**File:** `src/viewer/app.rs:1466-1477`
**Impact:** Stale camera/light data when switching files

```rust
// Only updates if scene has cameras AND list is not empty
if !scene.cameras.is_empty() {
    self.scene_cameras = ...  // Never clears old cameras
}
// Lights only set once (second condition prevents updates)
if !scene.lights.is_empty() && self.scene_lights.is_empty() { ... }
```

**Fix:** Clear `scene_cameras` and `scene_lights` before applying new scene data.

---

### 5. Viewer: Weak Vertex Hash Causes Missed Updates
**File:** `src/viewer/renderer/mod.rs:149-165`
**Impact:** Mesh updates may not be detected

Hash only checks `len + first_vertex + last_vertex`:
```rust
fn compute_vertex_hash(verts: &[Vertex]) -> u64 {
    let len = verts.len() as u64;
    let first = if !verts.is_empty() { ... } else { 0 };
    let last = if verts.len() > 1 { ... } else { 0 };
    len ^ (first << 16) ^ (last << 32)  // Middle vertices ignored!
}
```

**Fix:** Hash a sample of vertices throughout the array, not just endpoints.

---

### 6. Viewer: Unconditional Repaint Wastes CPU
**File:** `src/viewer/app.rs:1922`
**Impact:** 100% CPU usage even when idle

```rust
ctx.request_repaint();  // Called every frame unconditionally
```

**Fix:** Only call `request_repaint()` when animation is playing or content changes.

---

### 7. Python: `valid()` Always Returns True
**File:** `src/python/archive.rs:70-72`
**Impact:** Cannot detect invalid archives from Python

```rust
fn valid(&self) -> bool {
    true  // Always true!
}
```

**Fix:** Check actual archive validity state.

---

### 8. Python: String Array Parsing Incomplete
**File:** `src/python/properties.rs:509-512`
**Impact:** Multi-string arrays returned as single concatenated string

```rust
String | Wstring => {
    // String arrays are more complex, return as single string for now
    let s = std::string::String::from_utf8_lossy(data);
```

**Fix:** Parse null-terminated strings into `Vec<String>`.

---

### 9. Python: Silent Panics in Materials
**File:** `src/python/materials.rs:277-294`
**Impact:** Python interpreter crashes instead of raising exception

```rust
ShaderParamValue::Bool(v) => v.into_pyobject(py).unwrap().to_owned()...
```

**Fix:** Replace `.unwrap()` with `.map_err()` and return `PyResult`.

---

### 10. Python: Missing Constructors
**File:** `src/python/materials.rs:169, 92`
**Impact:** Cannot create `PyIMaterial` or `PyICollections` from Python

**Fix:** Add `#[new]` method accepting `PyIObject`.

---

### 11. Python: Performance - Re-traversal on Every Call
**File:** `src/python/object.rs:38-63`
**Impact:** O(n*d) performance for n operations at depth d

Every method call re-traverses the path from archive root.

**Fix:** Cache object references or use lazy evaluation.

---

## LOW SEVERITY ISSUES

### Dead Code Patterns

| File | Lines | Issue |
|------|-------|-------|
| `viewer/app.rs` | 429-475 | `show_tree_node_filtered` - old version kept as reference |
| `viewer/renderer/mod.rs` | 48, 82, 87 | Unused buffer fields (skybox_camera_layout, grid_model_buffer, env_uniform_buffer) |
| `viewer/renderer/mod.rs` | 139, 225-231, 240-249 | `name` fields in scene types never read |
| `viewer/renderer/mod.rs` | 1026, 1154 | Unused functions: `has_points`, `update_curves_transform` |
| `viewer/mesh_converter.rs` | 93, 107, 126, 134, 168, 180 | Multiple #[allow(dead_code)] fields |
| `ogawa/abc_impl.rs` | 33, 42 | `inner` and `cache` fields stored but not used after init |

### Code Duplication

| Pattern | Files | Occurrences |
|---------|-------|-------------|
| `.getPropertyByName(".geom")?.asCompound()?` | 11 files | 64 occurrences |
| Sample index clamping | app.rs, mesh_converter.rs | 12+ occurrences |
| Schema traversal methods | python/schemas.rs | 6 duplicate patterns |
| `bytemuck::try_cast_slice(&data).ok()?` | 3+ files | pattern throughout |

### TODO/Incomplete Features

| File | Line | Issue |
|------|------|-------|
| `viewer/viewport.rs` | 221 | TODO: Calculate scene bounds (hardcoded to ZERO, 5.0) |
| `viewer/renderer/mod.rs` | 247-249 | Point sprite widths "not yet used in rendering" |

### Minor Issues

| File | Line | Issue |
|------|------|-------|
| `viewer/renderer/mod.rs` | 1302 | `use_gbuffer = true` hardcoded, making conditional dead code |
| `python/write.rs` | 1366-1396 | `parse_data_type` missing several types (box3d, quat, normal3f) |
| `viewer/app.rs` | 272-280 | Unnecessary `std::mem::take` in every frame |

---

## Architecture Observations

### Good Patterns Found

1. **geom/util.rs** - Proper helper consolidation reducing duplication in geometry schemas
2. **core/traits.rs** - Clean trait hierarchy matching C++ Alembic API
3. **ogawa/writer/** - Well-structured modular design with clear separation

### Areas for Improvement

1. **Python bindings schema traversal** - Each schema duplicates traversal code
2. **Sample index clamping** - Should be a shared utility function
3. **Property access patterns** - Consider macro for `.geom` compound access

---

## Dataflow Diagrams

### Reading Pipeline
```
.abc File
    |
    v
+------------------+     +------------------+     +------------------+
|   IStreams       | --> | OgawaIArchive    | --> | OgawaArchive     |
|   (mmap)         |     | (binary parse)   |     |   Reader         |
+------------------+     +------------------+     +------------------+
                                                        |
                    +-----------------------------------+
                    |
    +---------------+---------------+---------------+
    |               |               |               |
    v               v               v               v
+--------+    +----------+    +----------+    +---------+
| IXform |    | IPolyMesh|    | ICurves  |    | ICamera |
+--------+    +----------+    +----------+    +---------+
    |               |               |               |
    +---------------+---------------+---------------+
                    |
                    v
            User Application / Viewer
```

### Writing Pipeline
```
User Application
    |
    v
+------------------+     +------------------+     +------------------+
| OPolyMesh/OXform | --> | OObject          | --> | OArchive         |
| (schema samples) |     | (hierarchy)      |     | (serialization)  |
+------------------+     +------------------+     +------------------+
                                                        |
                    +-----------------------------------+
                    |
                    v
+------------------+     +------------------+     +------------------+
| write_archive()  | --> | Deduplication    | --> | OStream          |
| (object tree)    |     | (hash map)       |     | (binary output)  |
+------------------+     +------------------+     +------------------+
                                                        |
                                                        v
                                                   .abc File
```

---

## Recommendations

### Priority 1 (Fix Now)
1. [ ] Fix material inheritance order in mesh_converter.rs
2. [ ] Fix `std::process::exit(0)` in app.rs
3. [ ] Fix matrix convention in Python write.rs

### Priority 2 (This Week)
4. [ ] Fix scene cameras/lights refresh logic
5. [ ] Improve vertex hash algorithm
6. [ ] Reduce CPU usage with conditional repaint
7. [ ] Fix Python `valid()` method
8. [ ] Complete string array parsing in Python

### Priority 3 (Technical Debt)
9. [ ] Remove dead code with #[allow(dead_code)]
10. [ ] Extract sample_idx clamping helper
11. [ ] Add missing Python constructors
12. [ ] Complete TODO in viewport.rs

### Priority 4 (Nice to Have)
13. [ ] Optimize Python object traversal
14. [ ] Refactor Python schema duplicates
15. [ ] Add missing data types to parse_data_type

---

## Files Modified Since Last Bug Hunt

Based on git history comparison:
- No critical regressions detected
- Previous fixes (PhantomData stubs, abc::OArchive stub, dead O* stubs) verified intact

---

## Test Coverage Notes

Current test files:
- `tests/read_files.rs` - basic read
- `tests/write_tests.rs` - write operations
- `tests/copy_heart_test.rs` - round-trip
- `tests/compare_gears_out.rs` - output comparison
- `python/tests/` - 6 test files

**Missing coverage:**
- Material inheritance resolution
- Viewer scene state management
- Python matrix transforms

---

## FIXES APPLIED (2026-01-20)

### HIGH Severity - Fixed

| # | Issue | File:Line | Fix |
|---|-------|-----------|-----|
| 1 | Material inheritance resolved AFTER applying | mesh_converter.rs:588 | Moved `resolve_material_inheritance()` BEFORE building lookup |
| 2 | Exit bypasses cleanup | app.rs:196 | Changed `std::process::exit(0)` to `ctx.send_viewport_cmd(ViewportCommand::Close)` |
| 3 | Matrix convention mismatch | python/write.rs:487 | Added `.transpose()` for row-major input, updated docs |

### MEDIUM Severity - Fixed

| # | Issue | File:Line | Fix |
|---|-------|-----------|-----|
| 4 | Scene cameras/lights never refresh | app.rs:1466 | Always update cameras/lights (removed conditional guards) |
| 5 | Unconditional repaint 100% CPU | app.rs:1922 | Only `request_repaint()` when `self.playing` |
| 6 | Python valid() always true | python/archive.rs:70 | Delegate to `self.inner.valid()` |

### Pre-Existing Python Binding Bugs - FIXED

| # | Issue | File:Line | Fix |
|---|-------|-----------|-----|
| 7 | `ICompoundProperty.property()` missing | python/object.rs:369, 428 | Changed to `getProperty()` |
| 8 | `OPointsSample::new` type mismatch | python/write.rs:762 | Convert `Vec<u64>` to `Vec<i64>` |
| 9 | `OSubDSample.with_scheme()` missing | python/write.rs:851 | Added builder method to struct |

### PyO3 Deprecation Warnings - FIXED

| # | Issue | Files | Fix |
|---|-------|-------|-----|
| 10 | `PyObject` deprecated | properties.rs, materials.rs | Changed to `Py<PyAny>` |
| 11 | `Python::with_gil` deprecated | properties.rs, materials.rs | Added `py: Python<'_>` to method signatures |

**Build Status:**
- ✅ Core library compiles
- ✅ Viewer feature compiles  
- ✅ Python feature compiles (0 warnings)

---

*Report generated: 2026-01-20*
*Analyzer: Claude Code Bug Hunt*
*Fixes applied: 2026-01-20*
