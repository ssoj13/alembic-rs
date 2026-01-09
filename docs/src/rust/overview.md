# Rust API Overview

The alembic-rs Rust API provides low-level and high-level interfaces for working with Alembic files.

## Module Structure

```
alembic
├── abc          # Core archive and object types
├── geom         # Geometry schemas (PolyMesh, Curves, etc.)
├── ogawa        # Ogawa format implementation
│   ├── reader   # Reading support
│   └── writer   # Writing support
├── core         # Core types (TimeSampling, MetaData, etc.)
├── material     # Material schema support
└── collection   # Collections support
```

## Key Types

### Reading

| Type | Description |
|------|-------------|
| `IArchive` | Input archive - entry point for reading |
| `IObject` | Input object in the hierarchy |
| `IPolyMesh` | PolyMesh geometry reader |
| `IXform` | Transform reader |
| `ICamera` | Camera reader |
| `ICurves` | Curves reader |
| `IPoints` | Points reader |
| `ISubD` | Subdivision surface reader |

### Writing

| Type | Description |
|------|-------------|
| `OArchive` | Output archive - entry point for writing |
| `OObject` | Generic output object |
| `OPolyMesh` | PolyMesh writer |
| `OXform` | Transform writer |
| `OCamera` | Camera writer |
| `OCurves` | Curves writer |
| `OPoints` | Points writer |
| `OSubD` | Subdivision surface writer |

## Error Handling

Most operations return `Result<T, AlembicError>`:

```rust
use alembic::abc::IArchive;

fn main() -> Result<(), alembic::AlembicError> {
    let archive = IArchive::open("file.abc")?;
    // ...
    Ok(())
}
```

## Lifetimes

Objects borrowed from an archive have a lifetime tied to the archive:

```rust
let archive = IArchive::open("file.abc")?;
let root = archive.root();  // Borrows from archive
let mesh = root.child(0)?;  // Borrows from root

// Archive must outlive all borrowed objects
```

## Thread Safety

- `IArchive` is `Send + Sync` - can be shared across threads
- Individual objects should be accessed from a single thread
- Consider cloning paths/data for multi-threaded processing
