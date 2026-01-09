# Schema Reference

This chapter documents all Alembic schemas supported by alembic-rs.

## Object Schemas

### Xform

Transform node for scene hierarchy.

**Schema name:** `AbcGeom_Xform_v3`

**Properties:**

| Name | Type | Description |
|------|------|-------------|
| `.vals` | Float64 Array | Transform operation values |
| `.ops` | UInt8 Array | Transform operation types |
| `.isNotConstantIdentity` | Bool | Whether transform changes |

**Transform Operations:**

| Op Code | Name | Values |
|---------|------|--------|
| 0 | ScaleX | 1 float |
| 1 | ScaleY | 1 float |
| 2 | ScaleZ | 1 float |
| 3 | Scale | 3 floats (x, y, z) |
| 4 | TranslateX | 1 float |
| 5 | TranslateY | 1 float |
| 6 | TranslateZ | 1 float |
| 7 | Translate | 3 floats (x, y, z) |
| 8 | RotateX | 1 float (degrees) |
| 9 | RotateY | 1 float (degrees) |
| 10 | RotateZ | 1 float (degrees) |
| 11 | RotateXYZ | 3 floats (degrees) |
| 12 | Matrix | 16 floats (4x4 matrix) |

**Rust API:**

```rust
use alembic_rs::ogawa::writer::OXform;

let mut xform = OXform::new("myTransform");
xform.add_translation_sample(1.0, 2.0, 3.0);
xform.add_rotation_sample(0.0, 45.0, 0.0);
xform.add_scale_sample(1.0, 1.0, 1.0);
xform.add_matrix_sample(&matrix_4x4);
```

### PolyMesh

Polygonal mesh geometry.

**Schema name:** `AbcGeom_PolyMesh_v1`

**Properties:**

| Name | Type | Scope | Description |
|------|------|-------|-------------|
| `P` | P3f Array | Vertex | Positions |
| `.faceIndices` | Int32 Array | - | Vertex indices |
| `.faceCounts` | Int32 Array | - | Vertices per face |
| `N` | N3f Array | FaceVarying | Normals |
| `.selfBnds` | Box3d | - | Bounding box |
| `uv` | V2f Array | FaceVarying | UV coordinates |
| `v` | V3f Array | Vertex | Velocities |

**Rust API:**

```rust
use alembic_rs::ogawa::writer::OPolyMesh;

let mut mesh = OPolyMesh::new("myMesh");
mesh.add_sample(&positions, &face_counts, &face_indices);
mesh.set_normals(&normals, &normal_indices);
mesh.set_uvs(&uvs, &uv_indices);
```

### SubD

Subdivision surface.

**Schema name:** `AbcGeom_SubD_v1`

**Additional Properties:**

| Name | Type | Description |
|------|------|-------------|
| `.scheme` | String | Subdivision scheme (catmull-clark, loop) |
| `.fvarIndices` | Int32 Array | Face-varying indices |
| `.fvarData` | Float32 Array | Face-varying data |
| `.cornerIndices` | Int32 Array | Corner vertex indices |
| `.cornerSharpnesses` | Float32 Array | Corner sharpness values |
| `.creaseIndices` | Int32 Array | Crease edge indices |
| `.creaseLengths` | Int32 Array | Crease lengths |
| `.creaseSharpnesses` | Float32 Array | Crease sharpness values |

### Curves

Curve geometry (hair, fur, splines).

**Schema name:** `AbcGeom_Curves_v1`

**Properties:**

| Name | Type | Description |
|------|------|-------------|
| `P` | P3f Array | Control point positions |
| `.nVertices` | Int32 Array | Vertices per curve |
| `.type` | UInt8 | Curve type (linear/cubic) |
| `.wrap` | UInt8 | Wrap mode |
| `.basis` | UInt8 | Basis type (Bezier, BSpline, etc.) |
| `width` | Float32 Array | Width per vertex |
| `N` | N3f Array | Normals |
| `uv` | V2f Array | UV parameters |

**Curve Types:**

| Value | Type |
|-------|------|
| 0 | Linear |
| 1 | Cubic |

**Basis Types:**

| Value | Basis |
|-------|-------|
| 0 | Bezier |
| 1 | B-Spline |
| 2 | Catmull-Rom |
| 3 | Hermite |
| 4 | Power |

### Points

Point cloud / particle system.

**Schema name:** `AbcGeom_Points_v1`

**Properties:**

| Name | Type | Description |
|------|------|-------------|
| `P` | P3f Array | Point positions |
| `.id` | UInt64 Array | Unique point IDs |
| `v` | V3f Array | Velocities |
| `width` | Float32 Array | Point radius |

### Camera

Camera with standard film/lens parameters.

**Schema name:** `AbcGeom_Camera_v1`

**Properties:**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `.core` | Float64[16] | - | Core camera parameters |
| `.focalLength` | Float64 | 35.0 | Focal length (mm) |
| `.horizontalAperture` | Float64 | 3.6 | Horizontal aperture (cm) |
| `.verticalAperture` | Float64 | 2.4 | Vertical aperture (cm) |
| `.horizontalFilmOffset` | Float64 | 0.0 | Horizontal offset (cm) |
| `.verticalFilmOffset` | Float64 | 0.0 | Vertical offset (cm) |
| `.lensSqueezeRatio` | Float64 | 1.0 | Anamorphic squeeze |
| `.nearClippingPlane` | Float64 | 0.1 | Near clip |
| `.farClippingPlane` | Float64 | 100000 | Far clip |
| `.fStop` | Float64 | 5.6 | F-number |
| `.focusDistance` | Float64 | 5.0 | Focus distance |
| `.shutterOpen` | Float64 | 0.0 | Shutter open time |
| `.shutterClose` | Float64 | 0.0208 | Shutter close time |

### FaceSet

Face grouping for material assignment.

**Schema name:** `AbcGeom_FaceSet_v1`

**Properties:**

| Name | Type | Description |
|------|------|-------------|
| `.faces` | Int32 Array | Face indices in this set |

### Light

Light source (limited support).

**Schema name:** `AbcGeom_Light_v1`

Typically uses custom properties for light parameters.

## Archive Metadata

**Properties:**

| Name | Description |
|------|-------------|
| `_ai_Application` | Application name |
| `_ai_dateWritten` | Write timestamp |
| `_ai_userDescription` | User description |
| `_ai_IncludeStreams` | Embedded streams |

```rust
archive.set_app_name("MyApp");
archive.set_description("Scene export");
```

## Visibility Property

**Property name:** `visible`

**Type:** Int8 (enum)

| Value | Meaning |
|-------|---------|
| -1 | Deferred (inherit from parent) |
| 0 | Hidden |
| 1 | Visible |

```rust
let mut vis = OVisibilityProperty::new();
vis.add_sample(Visibility::Visible);
mesh.add_visibility(vis);
```
