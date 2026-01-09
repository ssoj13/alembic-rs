# Python Examples for alembic_rs

This directory contains example scripts demonstrating how to use the alembic_rs Python library.

## Prerequisites

Install the alembic_rs package:

```bash
pip install alembic_rs
```

Or build from source:

```bash
cd /path/to/alembic-rs
maturin build --release
pip install target/wheels/alembic_rs-*.whl
```

## Examples

### basic_read.py
Demonstrates reading Alembic files:
- Opening archives
- Navigating object hierarchy
- Reading mesh data
- Accessing metadata

```bash
python basic_read.py path/to/file.abc
```

### basic_write.py
Demonstrates writing Alembic files:
- Creating archives
- Setting metadata
- Writing PolyMesh geometry
- Adding transforms

```bash
python basic_write.py
```

### animation.py
Demonstrates animated data:
- Time sampling setup
- Animated transforms (rotation)
- Animated geometry (deformation)
- Reading animated data

```bash
python animation.py
```

### hierarchy.py
Demonstrates scene hierarchies:
- Nested transform hierarchies
- Parent-child relationships
- Complex scene structures

```bash
python hierarchy.py
```

## Output

All examples that write files will create an `output/` directory with the generated `.abc` files. These files can be viewed in applications like Maya, Houdini, or Blender (with Alembic addon).

## API Overview

### Reading (IArchive)

```python
from alembic_rs import IArchive

archive = IArchive("path/to/file.abc")
top = archive.getTop()

for i in range(top.getNumChildren()):
    child = top.getChild(i)
    if child.isPolyMesh():
        sample = child.getPolyMeshSample(0)
        print(f"Vertices: {len(sample.positions)}")
```

### Writing (OArchive)

```python
import alembic_rs
from alembic_rs import OArchive

archive = OArchive.create("output.abc")
archive.setAppName("My App")

mesh = alembic_rs.Abc.OPolyMesh("mesh")
mesh.addSample(positions, face_counts, face_indices)
archive.writePolyMesh(mesh)
archive.close()
```

### With Transforms

```python
xform = alembic_rs.Abc.OXform("transform")
xform.addTranslationSample(1.0, 2.0, 3.0)

mesh = alembic_rs.Abc.OPolyMesh("mesh")
mesh.addSample(positions, face_counts, face_indices)

xform.addPolyMesh(mesh)
archive.writeXform(xform)
```

## More Information

See the full documentation at: https://github.com/your-repo/alembic-rs
