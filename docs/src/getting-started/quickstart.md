# Quick Start

This guide will help you get started with alembic-rs in just a few minutes.

## Reading an Alembic File

### Rust

```rust
use alembic::abc::IArchive;
use alembic::geom::IPolyMesh;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Open archive
    let archive = IArchive::open("model.abc")?;
    
    // Get root object
    let root = archive.root();
    println!("Archive has {} children", root.num_children());
    
    // Iterate children
    for child in root.children() {
        println!("Found: {} ({})", child.name(), child.full_name());
        
        // Check if it's a mesh
        if let Some(mesh) = IPolyMesh::new(&child) {
            let sample = mesh.get_sample(0)?;
            println!("  Vertices: {}", sample.positions.len());
            println!("  Faces: {}", sample.face_counts.len());
        }
    }
    
    Ok(())
}
```

### Python

```python
from alembic_rs import IArchive

# Open archive
archive = IArchive("model.abc")

# Get root object
top = archive.getTop()
print(f"Archive has {top.getNumChildren()} children")

# Iterate children
for i in range(top.getNumChildren()):
    child = top.getChild(i)
    print(f"Found: {child.getName()} ({child.getFullName()})")
    
    # Check if it's a mesh
    if child.isPolyMesh():
        sample = child.getPolyMeshSample(0)
        print(f"  Vertices: {len(sample.positions)}")
        print(f"  Faces: {len(sample.faceCounts)}")
```

## Writing an Alembic File

### Rust

```rust
use alembic::ogawa::writer::{OArchive, OPolyMesh, OPolyMeshSample};
use glam::Vec3;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create archive
    let mut archive = OArchive::create("output.abc")?;
    archive.set_app_name("My App");
    
    // Create mesh
    let positions = vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(1.0, 1.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    ];
    let face_counts = vec![4];
    let face_indices = vec![0, 1, 2, 3];
    
    let mut mesh = OPolyMesh::new("quad");
    mesh.add_sample(&OPolyMeshSample::new(positions, face_counts, face_indices));
    
    // Write
    archive.write_archive(&mesh.build())?;
    archive.close()?;
    
    Ok(())
}
```

### Python

```python
import alembic_rs
from alembic_rs import OArchive

# Create archive
archive = OArchive.create("output.abc")
archive.setAppName("My App")

# Create mesh
positions = [
    [0.0, 0.0, 0.0],
    [1.0, 0.0, 0.0],
    [1.0, 1.0, 0.0],
    [0.0, 1.0, 0.0],
]
face_counts = [4]
face_indices = [0, 1, 2, 3]

mesh = alembic_rs.Abc.OPolyMesh("quad")
mesh.addSample(positions, face_counts, face_indices)

# Write
archive.writePolyMesh(mesh)
archive.close()
```

## Next Steps

- [Reading Archives](../rust/reading.md) - Detailed reading guide
- [Writing Archives](../rust/writing.md) - Detailed writing guide
- [Geometry Types](../rust/geometry.md) - All supported geometry types
- [Animation](../rust/animation.md) - Working with animated data
