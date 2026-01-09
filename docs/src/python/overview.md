# Python API Overview

The alembic_rs Python API provides a Pythonic interface to the Alembic library.

## Module Structure

```python
alembic_rs
├── Abc           # Core types
│   ├── IArchive, IObject
│   ├── OArchive, OObject, OPolyMesh, OXform, ...
│   └── TimeSampling, PropertyInfo
└── AbcGeom       # Geometry types
    ├── PolyMeshSample, XformSample, ...
    └── ObjectVisibility
```

## Quick Import

```python
# Main types
from alembic_rs import IArchive, OArchive

# Access all types through submodules
import alembic_rs
mesh = alembic_rs.Abc.OPolyMesh("name")
```

## Key Classes

### Reading

| Class | Description |
|-------|-------------|
| `IArchive` | Open and read Alembic files |
| `IObject` | Navigate object hierarchy |
| `PolyMeshSample` | Mesh vertex/face data |
| `XformSample` | Transform matrix |
| `CameraSample` | Camera parameters |

### Writing

| Class | Description |
|-------|-------------|
| `OArchive` | Create Alembic files |
| `OObject` | Generic container object |
| `OPolyMesh` | Write polygon meshes |
| `OXform` | Write transforms |
| `OCurves` | Write curves |
| `OPoints` | Write point clouds |

## Type Conversions

### Positions/Vectors

Python lists of lists are converted to Alembic vectors:

```python
# Python list -> Vec3 array
positions = [
    [0.0, 0.0, 0.0],
    [1.0, 0.0, 0.0],
    [1.0, 1.0, 0.0],
]
```

### Matrices

4x4 matrices as nested lists:

```python
matrix = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 0.0, 1.0],
]
```

### NumPy Support

While not required, NumPy arrays work transparently:

```python
import numpy as np

positions = np.array([
    [0.0, 0.0, 0.0],
    [1.0, 0.0, 0.0],
], dtype=np.float32)

# Convert to list for alembic_rs
mesh.addSample(positions.tolist(), face_counts, face_indices)
```

## Error Handling

Python exceptions are raised for errors:

```python
from alembic_rs import IArchive

try:
    archive = IArchive("nonexistent.abc")
except IOError as e:
    print(f"Failed to open: {e}")

try:
    child = obj.getChild(999)
except ValueError as e:
    print(f"Invalid index: {e}")
```

## Context Managers

Archives support context managers:

```python
from alembic_rs import OArchive

with OArchive.create("output.abc") as archive:
    archive.setAppName("My App")
    # ... write data
# Automatically closed
```

## Iteration

Objects support iteration:

```python
top = archive.getTop()

# Iterate children
for child in top:
    print(child.getName())

# Get count
print(f"Children: {len(top)}")
```
