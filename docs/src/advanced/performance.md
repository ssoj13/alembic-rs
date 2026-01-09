# Performance Optimization

This chapter covers techniques for optimizing Alembic file reading and writing with alembic-rs.

## File Format: Ogawa vs HDF5

alembic-rs uses the Ogawa format exclusively, which offers significant performance advantages:

| Aspect | Ogawa | HDF5 |
|--------|-------|------|
| Read speed | Fast | Slower |
| Write speed | Fast | Slower |
| File size | Smaller | Larger |
| Threading | Thread-safe | Requires locks |
| Dependencies | None | libhdf5 |

## Memory Management

### Streaming Large Files

For large files, avoid loading everything into memory:

```rust
use alembic_rs::ogawa::reader::IArchive;

let archive = IArchive::open("large_file.abc")?;
let root = archive.root();

// Process objects one at a time
for child in root.children() {
    process_object(&child);
    // Memory is released after each iteration
}

fn process_object(obj: &IObject) {
    // Only load samples you need
    let mesh = obj.as_polymesh()?;
    
    // Get specific sample instead of all
    let sample = mesh.get_sample(0)?;
    
    // Process and discard
    process_sample(&sample);
}
```

### Lazy Loading

alembic-rs uses lazy loading - data is only read when accessed:

```rust
// Opening archive is fast - no geometry loaded yet
let archive = IArchive::open("file.abc")?;

// Hierarchy traversal is fast - only metadata
let mesh = find_mesh(&archive, "character/body")?;

// Data loaded only when accessed
let positions = mesh.get_positions(0)?;  // Loads positions
let normals = mesh.get_normals(0)?;      // Loads normals
```

## Writing Optimization

### Batch Operations

Group related writes together:

```rust
use alembic_rs::ogawa::writer::{OArchive, OPolyMesh, OXform};

let mut archive = OArchive::create("output.abc")?;

// Create all objects first
let mut meshes = Vec::new();
for i in 0..1000 {
    let mut mesh = OPolyMesh::new(&format!("mesh_{}", i));
    mesh.add_sample(&positions, &face_counts, &face_indices);
    meshes.push(mesh);
}

// Group under single parent
let mut root = OXform::new("meshes");
for mesh in meshes {
    root.add_polymesh(mesh);
}

// Single write operation
archive.add_xform(root);
archive.close()?;
```

### Minimize Sample Count

Only write samples when values change:

```rust
// BAD: Writing every frame even if static
for frame in 0..240 {
    mesh.add_sample(&same_positions, &face_counts, &face_indices);
}

// GOOD: Single sample for static geometry
mesh.add_sample(&positions, &face_counts, &face_indices);
// Alembic interpolates automatically
```

### Share Topology

When positions animate but topology doesn't:

```rust
// First sample with full topology
mesh.add_sample(&positions_frame0, &face_counts, &face_indices);

// Subsequent samples - positions only (topology reused)
mesh.add_positions_sample(&positions_frame1);
mesh.add_positions_sample(&positions_frame2);
```

## Reading Optimization

### Selective Loading

Only load what you need:

```rust
// Load only positions (skip normals, UVs)
let positions = mesh.get_positions(frame)?;

// Load bounding box only (very fast)
let bounds = mesh.get_bounds(frame)?;

// Check if animated before loading all samples
if mesh.is_constant() {
    let sample = mesh.get_sample(0)?;  // Single sample
} else {
    // Load samples for specific frames
    for frame in key_frames {
        let sample = mesh.get_sample(frame)?;
    }
}
```

### Parallel Processing

Process multiple objects in parallel:

```rust
use rayon::prelude::*;

let archive = IArchive::open("file.abc")?;
let objects: Vec<_> = archive.root().children().collect();

// Process in parallel
let results: Vec<_> = objects.par_iter()
    .filter_map(|obj| obj.as_polymesh().ok())
    .map(|mesh| process_mesh(&mesh))
    .collect();
```

### Caching

Cache frequently accessed data:

```rust
use std::collections::HashMap;

struct MeshCache {
    positions: HashMap<(String, usize), Vec<f32>>,
}

impl MeshCache {
    fn get_positions(&mut self, mesh: &IPolyMesh, frame: usize) -> &Vec<f32> {
        let key = (mesh.name().to_string(), frame);
        self.positions.entry(key).or_insert_with(|| {
            mesh.get_positions(frame).unwrap_or_default()
        })
    }
}
```

## File Size Optimization

### Compression

alembic-rs writes compressed data by default. For maximum compression:

```rust
let archive = OArchive::create_with_options("output.abc", ArchiveOptions {
    compression: Compression::Maximum,
    ..Default::default()
})?;
```

### Data Precision

Use appropriate precision:

```rust
// For bounding boxes and low-precision data
let bounds_f32 = OProperty::array_f32("bounds", bounds);

// For high-precision positions (default)
let positions_f64 = OProperty::array_f64("positions_hires", positions);
```

## Benchmarking

Profile your code:

```rust
use std::time::Instant;

let start = Instant::now();
let archive = IArchive::open("file.abc")?;
println!("Open: {:?}", start.elapsed());

let start = Instant::now();
let mesh = find_mesh(&archive)?;
let positions = mesh.get_positions(0)?;
println!("Load mesh: {:?}", start.elapsed());
```

## Python Performance Tips

```python
import alembic_rs
import time

# Use context managers for automatic cleanup
with alembic_rs.IArchive("file.abc") as archive:
    # Process data
    pass

# Batch operations
positions_list = []
for frame in range(100):
    positions_list.append(mesh.getPositions(frame))

# Process in bulk rather than one at a time
import numpy as np
all_positions = np.array(positions_list)
```
