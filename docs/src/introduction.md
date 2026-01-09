# Introduction

**alembic-rs** is a pure Rust implementation of the [Alembic](http://www.alembic.io/) file format, providing high-performance reading and writing of 3D scene data.

## What is Alembic?

Alembic is an open-source interchange framework for visual effects and animation. It's designed to efficiently store and share complex animated 3D data between different software packages like Maya, Houdini, Cinema 4D, and Blender.

Key features of Alembic:
- **Efficient storage** of animated geometry and transforms
- **Baked data** - no dependencies on procedural systems
- **Time sampling** for animation at any frame rate
- **Hierarchical scene structure**
- **Support for various geometry types** (PolyMesh, SubD, Curves, Points, etc.)

## Why alembic-rs?

- **Pure Rust** - No C++ dependencies or build complexity
- **Safe** - Memory-safe implementation with Rust's guarantees
- **Fast** - Optimized for performance with zero-copy where possible
- **Cross-platform** - Works on Windows, macOS, and Linux
- **Python bindings** - Full Python API via PyO3

## Features

### Reading
- Full support for Ogawa archive format
- All standard geometry schemas (PolyMesh, SubD, Curves, Points, Camera, Xform, etc.)
- Property reading (scalar, array, compound)
- Time sampling and animation
- Materials and collections

### Writing
- Create new Ogawa archives
- Write geometry with animation
- Custom properties
- Deduplication for efficient storage
- Compression support

## Quick Example

### Rust

```rust
use alembic::abc::IArchive;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let archive = IArchive::open("scene.abc")?;
    let root = archive.root();
    
    for child in root.children() {
        println!("{}: {}", child.name(), child.meta_data().get("schema").unwrap_or(""));
    }
    
    Ok(())
}
```

### Python

```python
from alembic_rs import IArchive

archive = IArchive("scene.abc")
top = archive.getTop()

for i in range(top.getNumChildren()):
    child = top.getChild(i)
    print(f"{child.getName()}: {child.getSchemaType()}")
```

## License

alembic-rs is dual-licensed under MIT and Apache 2.0 licenses.
