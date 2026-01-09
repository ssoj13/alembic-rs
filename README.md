# alembic-rs

Rust implementation of the [Alembic](https://github.com/alembic/alembic) (.abc) 3D interchange format.

## Overview

Alembic is an open computer graphics interchange framework developed by Sony Pictures Imageworks and Industrial Light & Magic. It distills complex, animated scenes into baked geometric results.

This library provides native Rust support for reading Alembic files in the Ogawa binary format.

## Features

- **Archive reading** - Open and parse .abc files
- **Object hierarchy** - Traverse scene graph with parent/child relationships
- **Time sampling** - Support for uniform, cyclic, and acyclic time sampling
- **Geometry schemas**:
  - `IXform` - Transform nodes with matrix computation
  - `IPolyMesh` - Polygon meshes with positions, normals, UVs
  - `ICurves` - NURBS/Bezier/linear curves
  - `IPoints` - Point clouds / particles
  - `ISubD` - Subdivision surfaces with creases
  - `ICamera` - Camera parameters with FOV

## Example

```rust
use alembic::abc::IArchive;
use alembic::geom::{IXform, IPolyMesh};

let archive = IArchive::open("scene.abc")?;
let root = archive.root();

for child in root.children() {
    if let Some(mesh) = IPolyMesh::new(&child) {
        let sample = mesh.get_sample(0)?;
        println!("{}: {} vertices", mesh.name(), sample.num_vertices());
    }
}
```

## Status

**Work in progress.** Reading is mostly implemented, writing is planned.

## License

MIT

## References

- [Alembic (original C++)](https://github.com/alembic/alembic)
- [Alembic Documentation](http://www.alembic.io/)
