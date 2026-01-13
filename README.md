# alembic-rs

> **Experimental** pure Rust implementation of the [Alembic](http://www.alembic.io/) file format.
>
> An experimental pure Rust implementation of the Alembic file format, developed with AI assistance as an exploration of specification-driven development.

## Why?

- **Native Alembic** - No C++ dependencies, just `cargo build`
- **Memory safe** - Rust's guarantees for handling complex binary formats
- **Cross-platform** - Windows, macOS, Linux out of the box

## Features

- **Read & Write** - Full Ogawa archive support
- **All Geometry Types** - PolyMesh, SubD, Curves, Points, Camera, Xform, NuPatch, Light, FaceSet
- **Animation** - Time sampling, keyframes, deformation
- **Python Bindings** - Complete Python API via PyO3
- **Binary Compatible** - Hash-compatible output with C++ Alembic
- **3D Viewer** - Built-in GPU-accelerated viewer with PBR rendering
- **CLI Tool** - `alembic` command-line utility for inspection and conversion

## Status

This is an experimental implementation. While it passes compatibility tests with C++ Alembic, edge cases may exist. Use in production at your own discretion.

## CLI Tool

The `alembic` binary provides file inspection and a 3D viewer:

```bash
alembic view model.abc      # Open in 3D viewer (Esc to exit)
alembic info scene.abc      # Archive info and object counts
alembic tree character.abc  # Object hierarchy
alembic dump scene.abc      # Dump xform transforms
alembic copy in.abc out.abc # Round-trip copy test
```

### Viewer Features
- Orbit camera (LMB drag, scroll to zoom)
- PBR rendering with environment lighting
- Wireframe, X-Ray, shadows toggles
- Animation timeline scrubbing
- Settings persist between sessions

## Installation

### Rust Library

```toml
[dependencies]
alembic_core = "0.1"
```

### Python

```bash
pip install alembic_rs
```

Or build from source:
```bash
pip install maturin
maturin build --release
pip install target/wheels/alembic_rs-*.whl
```

## Quick Start

### Reading (Rust)

```rust
use alembic_core::abc::IArchive;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let archive = IArchive::open("scene.abc")?;
    let root = archive.root();
    
    for child in root.children() {
        println!("{}: {}", child.name(), child.schema_type());
    }
    
    Ok(())
}
```

### Reading (Python)

```python
from alembic_rs import IArchive, IPolyMesh

archive = IArchive("scene.abc")
root = archive.getTop()

for child in root:
    print(f"{child.getName()}: {child.getSchemaType()}")
    
    if child.isPolyMesh():
        # Direct access
        sample = child.getPolyMeshSample(0)
        
        # Or original-style API
        mesh = IPolyMesh(child)
        sample = mesh.getSchema().getValue()
        
        print(f"  Vertices: {len(sample.positions)}")
```

### Writing (Rust)

```rust
use alembic_core::ogawa::writer::{OArchive, OPolyMesh};

let mut archive = OArchive::create("output.abc")?;
archive.set_app_name("MyApp");

let mut mesh = OPolyMesh::new("cube");
mesh.add_sample(&positions, &face_counts, &face_indices);

archive.write_archive(&mesh.build())?;
```

### Writing (Python)

```python
from alembic_rs import OArchive, OPolyMesh, OObject

archive = OArchive.create("output.abc")
archive.setAppName("MyApp")

mesh = OPolyMesh("cube")
mesh.addSample(positions, face_counts, face_indices)

root = OObject("")
root.addPolyMesh(mesh)
archive.writeArchive(root)
```

## Supported Schemas

| Schema | Read | Write | Description |
|--------|------|-------|-------------|
| Xform | Yes | Yes | Transforms |
| PolyMesh | Yes | Yes | Polygonal meshes |
| SubD | Yes | Yes | Subdivision surfaces |
| Curves | Yes | Yes | Splines, hair, fur |
| Points | Yes | Yes | Particles, point clouds |
| Camera | Yes | Yes | Cameras |
| NuPatch | Yes | Yes | NURBS surfaces |
| Light | Yes | Yes | Lights |
| FaceSet | Yes | Yes | Material groups |
| Material | Yes | Yes | Material references |
| Collections | Yes | Yes | Object groups |

## Documentation

Full documentation available at [docs/](./docs/) or build with:

```bash
cd docs
mdbook build
```

## Binary Compatibility

alembic-rs produces files that are binary-compatible with C++ Alembic:

- SpookyHash V2 for object/property headers
- MurmurHash3 for sample digests
- Identical hash computation for deduplication
- Compatible metadata format

## License

BSD-3-Clause (same as Alembic)

## Credits

Based on the [Alembic](https://github.com/alembic/alembic) specification by Sony Pictures Imageworks and ILM.

---

*This project is not affiliated with or endorsed by the original Alembic developers.*
