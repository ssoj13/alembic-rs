# Reading Archives

## Opening an Archive

```rust
use alembic::abc::IArchive;

let archive = IArchive::open("scene.abc")?;

// Get archive metadata
println!("Name: {}", archive.name());
if let Some(app) = archive.app_name() {
    println!("Created by: {}", app);
}
```

## Navigating the Hierarchy

### Root Object

```rust
let root = archive.root();
println!("Root: {}", root.full_name());  // "/"
println!("Children: {}", root.num_children());
```

### Iterating Children

```rust
// By index
for i in 0..root.num_children() {
    if let Some(child) = root.child(i) {
        println!("{}", child.name());
    }
}

// By iterator
for child in root.children() {
    println!("{}", child.name());
}

// By name
if let Some(obj) = root.child_by_name("mesh") {
    println!("Found mesh!");
}
```

### Finding Objects by Path

```rust
if archive.has_object("/group/mesh") {
    // Object exists
}
```

## Reading Geometry

### PolyMesh

```rust
use alembic::geom::IPolyMesh;

if let Some(mesh) = IPolyMesh::new(&object) {
    println!("Samples: {}", mesh.num_samples());
    
    let sample = mesh.get_sample(0)?;
    
    // Vertex positions
    for pos in &sample.positions {
        println!("  ({}, {}, {})", pos.x, pos.y, pos.z);
    }
    
    // Face topology
    println!("Faces: {}", sample.face_counts.len());
    
    // Optional data
    if let Some(normals) = &sample.normals {
        println!("Has normals: {}", normals.len());
    }
    if let Some(uvs) = &sample.uvs {
        println!("Has UVs: {}", uvs.len());
    }
}
```

### Transform (Xform)

```rust
use alembic::geom::IXform;

if let Some(xform) = IXform::new(&object) {
    let sample = xform.get_sample(0)?;
    
    // Get 4x4 matrix
    let matrix = sample.matrix();
    
    // Check inheritance
    if sample.inherits {
        println!("Inherits parent transform");
    }
}
```

### Camera

```rust
use alembic::geom::ICamera;

if let Some(camera) = ICamera::new(&object) {
    let sample = camera.get_sample(0)?;
    
    println!("Focal length: {}", sample.focal_length);
    println!("Aperture: {}x{}", 
        sample.horizontal_aperture,
        sample.vertical_aperture);
    println!("Near/Far: {}/{}", 
        sample.near_clipping_plane,
        sample.far_clipping_plane);
}
```

## Reading Properties

### Scalar Properties

```rust
use alembic::abc::ITypedScalarProperty;

let props = object.properties();
if let Some(prop) = props.property_by_name("customValue") {
    if let Some(scalar) = prop.as_scalar() {
        // Read as f32
        let typed: ITypedScalarProperty<f32> = scalar.typed()?;
        let value = typed.get(0)?;
        println!("Value: {}", value);
    }
}
```

### Array Properties

```rust
use alembic::abc::ITypedArrayProperty;

if let Some(prop) = props.property_by_name("colors") {
    if let Some(array) = prop.as_array() {
        let typed: ITypedArrayProperty<[f32; 3]> = array.typed()?;
        let colors = typed.get(0)?;
        for color in colors {
            println!("RGB: {:?}", color);
        }
    }
}
```

## Time Sampling

```rust
// Get time sampling for the archive
let num_ts = archive.num_time_samplings();
for i in 0..num_ts {
    if let Some(ts) = archive.time_sampling(i) {
        println!("TimeSampling {}: {:?}", i, ts);
    }
}

// Find sample index for a specific time
let sample_idx = mesh.get_sample_index_for_time(1.0 / 24.0)?;
```
