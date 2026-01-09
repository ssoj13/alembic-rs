# Writing Archives (Python)

## Creating an Archive

```python
from alembic_rs import OArchive
import alembic_rs

# Create archive
archive = OArchive.create("output.abc")

# Set metadata
archive.setAppName("My Application")
archive.setDescription("Scene export")
archive.setDateWritten("2024-01-15")
archive.setDccFps(24.0)

# Configuration
archive.setCompressionHint(6)  # 0-9, -1=disabled
archive.setDedupEnabled(True)
```

## Writing Geometry

### PolyMesh

```python
# Create mesh
mesh = alembic_rs.Abc.OPolyMesh("cube")

# Define geometry
positions = [
    [-1.0, -1.0, -1.0],
    [ 1.0, -1.0, -1.0],
    [ 1.0,  1.0, -1.0],
    [-1.0,  1.0, -1.0],
    [-1.0, -1.0,  1.0],
    [ 1.0, -1.0,  1.0],
    [ 1.0,  1.0,  1.0],
    [-1.0,  1.0,  1.0],
]

face_counts = [4, 4, 4, 4, 4, 4]  # 6 quads

face_indices = [
    0, 3, 2, 1,  # back
    4, 5, 6, 7,  # front
    0, 1, 5, 4,  # bottom
    2, 3, 7, 6,  # top
    0, 4, 7, 3,  # left
    1, 2, 6, 5,  # right
]

# Add sample (required)
mesh.addSample(positions, face_counts, face_indices)

# Write
archive.writePolyMesh(mesh)
archive.close()
```

### With Normals and UVs

```python
normals = [
    [0.0, 0.0, -1.0],  # Per-vertex normals
    # ...
]

uvs = [
    [0.0, 0.0],  # Per-vertex UVs
    [1.0, 0.0],
    # ...
]

mesh.addSample(
    positions, 
    face_counts, 
    face_indices,
    normals=normals,
    uvs=uvs
)
```

### Transform (Xform)

```python
xform = alembic_rs.Abc.OXform("group")

# Identity transform
xform.addIdentitySample()

# Translation
xform.addTranslationSample(1.0, 2.0, 3.0)

# Scale
xform.addScaleSample(2.0, 2.0, 2.0)

# Matrix
import math
angle = math.pi / 4
matrix = [
    [math.cos(angle), 0.0, math.sin(angle), 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [-math.sin(angle), 0.0, math.cos(angle), 0.0],
    [0.0, 0.0, 0.0, 1.0],
]
xform.addMatrixSample(matrix, inherits=True)
```

### Curves

```python
curves = alembic_rs.Abc.OCurves("hair")

# All curve positions concatenated
positions = [
    # Curve 0
    [0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 2.0, 0.0],
    # Curve 1
    [1.0, 0.0, 0.0], [1.0, 1.0, 0.2], [1.0, 2.0, 0.0],
]

# Vertices per curve
num_vertices = [3, 3]

curves.addSample(
    positions,
    num_vertices,
    curve_type="linear",  # or "cubic"
    wrap="nonperiodic",   # or "periodic"
    basis="nobasis",      # or "bezier", "bspline", "catmullrom"
    widths=[0.01, 0.01, 0.01, 0.01, 0.01, 0.01],
)
```

### Points

```python
points = alembic_rs.Abc.OPoints("particles")

positions = [
    [0.0, 0.0, 0.0],
    [1.0, 0.5, 0.2],
    [2.0, 0.1, 0.8],
]

ids = [0, 1, 2]  # Unique IDs for tracking

velocities = [
    [0.1, 0.0, 0.0],
    [0.0, 0.1, 0.0],
    [0.0, 0.0, 0.1],
]

widths = [0.1, 0.1, 0.1]

points.addSample(positions, ids, velocities=velocities, widths=widths)
```

## Building Hierarchies

```python
# Create hierarchy
root = alembic_rs.Abc.OObject("scene")

# Transform with children
group = alembic_rs.Abc.OXform("group")
group.addTranslationSample(0.0, 1.0, 0.0)

# Add mesh to transform
mesh = alembic_rs.Abc.OPolyMesh("mesh")
mesh.addSample(positions, face_counts, face_indices)
group.addPolyMesh(mesh)

# Add transform to root
root.addXform(group)

# Write hierarchy
archive.writeArchive(root)
```

## Animation

### Time Sampling

```python
# Uniform time sampling (e.g., 24 fps)
ts_index = archive.addUniformTimeSampling(24.0, start_time=0.0)

# Acyclic (arbitrary times)
times = [0.0, 0.5, 1.0, 2.0, 4.0]
ts_index = archive.addAcyclicTimeSampling(times)

# Cyclic
ts_index = archive.addCyclicTimeSampling(
    time_per_cycle=1.0,
    times=[0.0, 0.25, 0.5, 0.75]
)
```

### Animated Transform

```python
import math

xform = alembic_rs.Abc.OXform("spinner")

# 48 frames at 24 fps = 2 seconds
for frame in range(48):
    angle = (frame / 48.0) * 2 * math.pi
    
    cos_a = math.cos(angle)
    sin_a = math.sin(angle)
    
    matrix = [
        [cos_a, 0.0, sin_a, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [-sin_a, 0.0, cos_a, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
    
    xform.addMatrixSample(matrix, inherits=True)
```

### Animated Mesh

```python
mesh = alembic_rs.Abc.OPolyMesh("deforming")

# Topology stays constant
face_counts = [4]
face_indices = [0, 1, 2, 3]

# Deform positions over time
for frame in range(24):
    t = frame / 24.0
    offset = math.sin(t * math.pi * 2) * 0.5
    
    positions = [
        [0.0, offset, 0.0],
        [1.0, offset, 0.0],
        [1.0, 1.0 + offset, 0.0],
        [0.0, 1.0 + offset, 0.0],
    ]
    
    mesh.addSample(positions, face_counts, face_indices)
```

## Context Manager

```python
with OArchive.create("output.abc") as archive:
    archive.setAppName("My App")
    
    mesh = alembic_rs.Abc.OPolyMesh("mesh")
    mesh.addSample(positions, face_counts, face_indices)
    archive.writePolyMesh(mesh)
    
# Automatically closed
```
