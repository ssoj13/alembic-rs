# Data Types Reference

This chapter documents all data types supported by alembic-rs.

## Primitive Types

| Type | Rust | Python | Description |
|------|------|--------|-------------|
| Bool | `bool` | `bool` | Boolean value |
| UInt8 | `u8` | `int` | Unsigned 8-bit integer |
| Int8 | `i8` | `int` | Signed 8-bit integer |
| UInt16 | `u16` | `int` | Unsigned 16-bit integer |
| Int16 | `i16` | `int` | Signed 16-bit integer |
| UInt32 | `u32` | `int` | Unsigned 32-bit integer |
| Int32 | `i32` | `int` | Signed 32-bit integer |
| UInt64 | `u64` | `int` | Unsigned 64-bit integer |
| Int64 | `i64` | `int` | Signed 64-bit integer |
| Float16 | `f16` | `float` | 16-bit floating point |
| Float32 | `f32` | `float` | 32-bit floating point |
| Float64 | `f64` | `float` | 64-bit floating point |
| String | `String` | `str` | UTF-8 string |

## Compound Types

### Vec2

2D vector types:

| Type | Components | Rust | Description |
|------|------------|------|-------------|
| V2s | i16, i16 | `[i16; 2]` | Short vector |
| V2i | i32, i32 | `[i32; 2]` | Integer vector |
| V2f | f32, f32 | `[f32; 2]` | Float vector |
| V2d | f64, f64 | `[f64; 2]` | Double vector |

### Vec3

3D vector types:

| Type | Components | Rust | Description |
|------|------------|------|-------------|
| V3s | i16 x3 | `[i16; 3]` | Short vector |
| V3i | i32 x3 | `[i32; 3]` | Integer vector |
| V3f | f32 x3 | `[f32; 3]` | Float vector |
| V3d | f64 x3 | `[f64; 3]` | Double vector |
| P3s | i16 x3 | `[i16; 3]` | Short point |
| P3i | i32 x3 | `[i32; 3]` | Integer point |
| P3f | f32 x3 | `[f32; 3]` | Float point |
| P3d | f64 x3 | `[f64; 3]` | Double point |
| N3f | f32 x3 | `[f32; 3]` | Float normal |
| N3d | f64 x3 | `[f64; 3]` | Double normal |

### Color

Color types:

| Type | Components | Rust | Description |
|------|------------|------|-------------|
| C3h | f16 x3 | `[f16; 3]` | Half RGB |
| C3f | f32 x3 | `[f32; 3]` | Float RGB |
| C3c | u8 x3 | `[u8; 3]` | Byte RGB |
| C4h | f16 x4 | `[f16; 4]` | Half RGBA |
| C4f | f32 x4 | `[f32; 4]` | Float RGBA |
| C4c | u8 x4 | `[u8; 4]` | Byte RGBA |

### Matrix

Matrix types:

| Type | Size | Rust | Description |
|------|------|------|-------------|
| M33f | 3x3 | `[[f32; 3]; 3]` | Float 3x3 matrix |
| M33d | 3x3 | `[[f64; 3]; 3]` | Double 3x3 matrix |
| M44f | 4x4 | `[[f32; 4]; 4]` | Float 4x4 matrix |
| M44d | 4x4 | `[[f64; 4]; 4]` | Double 4x4 matrix |

### Quaternion

| Type | Components | Rust | Description |
|------|------------|------|-------------|
| Quatf | f32 x4 | `[f32; 4]` | Float quaternion (w, x, y, z) |
| Quatd | f64 x4 | `[f64; 4]` | Double quaternion (w, x, y, z) |

### Box

Bounding box types:

| Type | Components | Rust | Description |
|------|------------|------|-------------|
| Box2s | V2s x2 | `[[i16; 2]; 2]` | 2D short box |
| Box2i | V2i x2 | `[[i32; 2]; 2]` | 2D integer box |
| Box2f | V2f x2 | `[[f32; 2]; 2]` | 2D float box |
| Box2d | V2d x2 | `[[f64; 2]; 2]` | 2D double box |
| Box3s | V3s x2 | `[[i16; 3]; 2]` | 3D short box |
| Box3i | V3i x2 | `[[i32; 3]; 2]` | 3D integer box |
| Box3f | V3f x2 | `[[f32; 3]; 2]` | 3D float box |
| Box3d | V3d x2 | `[[f64; 3]; 2]` | 3D double box |

## Geometry Types

### PolyMesh

```rust
pub struct PolyMeshSample {
    pub positions: Vec<[f32; 3]>,    // P3f array
    pub face_counts: Vec<i32>,       // Int32 array
    pub face_indices: Vec<i32>,      // Int32 array
    pub normals: Option<Vec<[f32; 3]>>,  // N3f array (indexed)
    pub uvs: Option<Vec<[f32; 2]>>,  // V2f array (indexed)
    pub velocities: Option<Vec<[f32; 3]>>,  // V3f array
}
```

### Curves

```rust
pub struct CurvesSample {
    pub positions: Vec<[f32; 3]>,    // P3f array
    pub num_vertices: Vec<i32>,      // Vertices per curve
    pub curve_type: CurveType,       // Linear, Cubic, etc.
    pub wrap: CurveWrap,             // NonPeriodic, Periodic
    pub basis: CurveBasis,           // Bezier, BSpline, etc.
    pub widths: Option<Vec<f32>>,    // Width per vertex
}

pub enum CurveType { Linear, Cubic }
pub enum CurveWrap { NonPeriodic, Periodic }
pub enum CurveBasis { Bezier, BSpline, CatmullRom, Hermite, Power }
```

### Points

```rust
pub struct PointsSample {
    pub positions: Vec<[f32; 3]>,    // P3f array
    pub ids: Option<Vec<u64>>,       // Unique point IDs
    pub velocities: Option<Vec<[f32; 3]>>,  // V3f array
    pub widths: Option<Vec<f32>>,    // Point radius
}
```

### Camera

```rust
pub struct CameraSample {
    pub focal_length: f64,           // mm
    pub horizontal_aperture: f64,    // cm
    pub vertical_aperture: f64,      // cm
    pub horizontal_film_offset: f64, // cm
    pub vertical_film_offset: f64,   // cm
    pub lens_squeeze_ratio: f64,     // Anamorphic squeeze
    pub near_clip: f64,              // Near clipping plane
    pub far_clip: f64,               // Far clipping plane
    pub fstop: f64,                  // f-number
    pub focus_distance: f64,         // Focus distance
    pub shutter_open: f64,           // Shutter open time
    pub shutter_close: f64,          // Shutter close time
}
```

## Property Scopes

| Scope | Description | Example |
|-------|-------------|---------|
| Constant | Same value for all samples | Material name |
| Uniform | One value per face | Face materials |
| Varying | One value per face vertex | Per-face-vertex colors |
| Vertex | One value per vertex | Position, normal |
| FaceVarying | One value per face-vertex | UVs (can differ per face) |

## Time Sampling

```rust
pub struct TimeSampling {
    pub time_per_cycle: f64,         // Duration of one cycle
    pub start_time: f64,             // First sample time
    pub sample_times: Vec<f64>,      // Times within cycle
}

// Common sampling patterns:
// Uniform: 24fps -> time_per_cycle = 1/24
// Cyclic: Looping animation
// Acyclic: Irregular sample times
```
