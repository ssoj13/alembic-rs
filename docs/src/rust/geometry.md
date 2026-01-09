# Geometry Types

alembic-rs supports all standard Alembic geometry schemas.

## PolyMesh

Polygonal meshes with arbitrary face topology.

```rust
use alembic::geom::{IPolyMesh, PolyMeshSample};

if let Some(mesh) = IPolyMesh::new(&object) {
    let sample: PolyMeshSample = mesh.get_sample(0)?;
    
    // Required data
    let positions: &[Vec3] = &sample.positions;
    let face_counts: &[i32] = &sample.face_counts;
    let face_indices: &[i32] = &sample.face_indices;
    
    // Optional data
    let velocities: Option<&[Vec3]> = sample.velocities.as_deref();
    let normals: Option<&[Vec3]> = sample.normals.as_deref();
    let uvs: Option<&[Vec2]> = sample.uvs.as_deref();
    let bounds: Option<Box3> = sample.self_bounds;
}
```

## SubD (Subdivision Surface)

Subdivision surfaces with creases and corners.

```rust
use alembic::geom::{ISubD, SubDSample, SubdivisionScheme};

if let Some(subd) = ISubD::new(&object) {
    let sample: SubDSample = subd.get_sample(0)?;
    
    // Base mesh data (same as PolyMesh)
    let positions = &sample.positions;
    let face_counts = &sample.face_counts;
    let face_indices = &sample.face_indices;
    
    // Subdivision scheme
    let scheme: SubdivisionScheme = sample.scheme;
    // CatmullClark, Loop, Bilinear
    
    // Creases
    let crease_indices = &sample.crease_indices;
    let crease_lengths = &sample.crease_lengths;
    let crease_sharpnesses = &sample.crease_sharpnesses;
    
    // Corners
    let corner_indices = &sample.corner_indices;
    let corner_sharpnesses = &sample.corner_sharpnesses;
    
    // Holes
    let holes = &sample.holes;
}
```

## Curves

Curves including hair, NURBS curves, and bezier curves.

```rust
use alembic::geom::{ICurves, CurvesSample, CurveType, BasisType};

if let Some(curves) = ICurves::new(&object) {
    let sample: CurvesSample = curves.get_sample(0)?;
    
    // Positions for all curves (concatenated)
    let positions = &sample.positions;
    
    // Number of vertices per curve
    let num_vertices = &sample.num_vertices;
    
    // Curve type
    let curve_type: CurveType = sample.curve_type;
    // Linear, Cubic
    
    // Basis (for cubic)
    let basis: BasisType = sample.basis;
    // NoBasis, Bezier, Bspline, CatmullRom, Hermite
    
    // Optional
    let widths = sample.widths.as_deref();
    let knots = sample.knots.as_deref();
    let orders = sample.orders.as_deref();
}
```

## Points

Point clouds and particle systems.

```rust
use alembic::geom::{IPoints, PointsSample};

if let Some(points) = IPoints::new(&object) {
    let sample: PointsSample = points.get_sample(0)?;
    
    // Positions
    let positions = &sample.positions;
    
    // Point IDs (for tracking across frames)
    let ids: &[u64] = &sample.ids;
    
    // Optional
    let velocities = sample.velocities.as_deref();
    let widths = sample.widths.as_deref();
}
```

## Xform (Transform)

Transformation matrices.

```rust
use alembic::geom::{IXform, XformSample};

if let Some(xform) = IXform::new(&object) {
    let sample: XformSample = xform.get_sample(0)?;
    
    // 4x4 transformation matrix
    let matrix: Mat4 = sample.matrix();
    
    // Does this inherit from parent?
    let inherits: bool = sample.inherits;
}
```

## Camera

Camera with lens and film parameters.

```rust
use alembic::geom::{ICamera, CameraSample};

if let Some(camera) = ICamera::new(&object) {
    let sample: CameraSample = camera.get_sample(0)?;
    
    // Lens
    let focal_length: f64 = sample.focal_length;
    let lens_squeeze: f64 = sample.lens_squeeze_ratio;
    
    // Film/sensor
    let h_aperture: f64 = sample.horizontal_aperture;
    let v_aperture: f64 = sample.vertical_aperture;
    let h_offset: f64 = sample.horizontal_film_offset;
    let v_offset: f64 = sample.vertical_film_offset;
    
    // Clipping
    let near: f64 = sample.near_clipping_plane;
    let far: f64 = sample.far_clipping_plane;
    
    // DOF
    let f_stop: f64 = sample.f_stop;
    let focus: f64 = sample.focus_distance;
    
    // Shutter
    let shutter_open: f64 = sample.shutter_open;
    let shutter_close: f64 = sample.shutter_close;
}
```

## NuPatch (NURBS Surface)

NURBS patches.

```rust
use alembic::geom::{INuPatch, NuPatchSample};

if let Some(nupatch) = INuPatch::new(&object) {
    let sample: NuPatchSample = nupatch.get_sample(0)?;
    
    // Control vertices
    let positions = &sample.positions;
    
    // Surface dimensions
    let num_u: i32 = sample.num_u;
    let num_v: i32 = sample.num_v;
    
    // Orders (degree + 1)
    let u_order: i32 = sample.u_order;
    let v_order: i32 = sample.v_order;
    
    // Knot vectors
    let u_knots = &sample.u_knots;
    let v_knots = &sample.v_knots;
    
    // Optional weights (for rational NURBS)
    let weights = sample.position_weights.as_deref();
}
```

## Light

Light with camera-like parameters.

```rust
use alembic::geom::{ILight, LightSample};

if let Some(light) = ILight::new(&object) {
    let sample: LightSample = light.get_sample(0)?;
    
    // Light uses camera parameters for consistency
    let camera_params = &sample.camera;
    
    // Child bounds
    let bounds = sample.child_bounds;
}
```

## FaceSet

Named face groups within a mesh.

```rust
use alembic::geom::{IFaceSet, FaceSetSample};

// Find FaceSets under a mesh
for child in mesh_object.children() {
    if let Some(faceset) = IFaceSet::new(&child) {
        let sample: FaceSetSample = faceset.get_sample(0)?;
        
        // Indices of faces in this set
        let faces: &[i32] = &sample.faces;
        
        println!("FaceSet '{}' contains {} faces", 
            child.name(), faces.len());
    }
}
```
