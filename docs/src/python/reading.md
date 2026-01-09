# Reading Archives (Python)

## Opening an Archive

```python
from alembic_rs import IArchive

# Open archive
archive = IArchive("scene.abc")

# Check validity
if archive.valid():
    print(f"Opened: {archive.getName()}")
```

## Archive Metadata

```python
# Application info
print(f"Created by: {archive.getAppName()}")
print(f"Date: {archive.getDateWritten()}")
print(f"Description: {archive.getUserDescription()}")
print(f"DCC FPS: {archive.getDccFps()}")

# Custom metadata
keys = archive.getMetadataKeys()
for key in keys:
    print(f"{key}: {archive.getMetadata(key)}")
```

## Navigating the Hierarchy

### Getting the Root

```python
top = archive.getTop()
print(f"Root: {top.getFullName()}")
print(f"Children: {top.getNumChildren()}")
```

### Iterating Children

```python
# By index
for i in range(top.getNumChildren()):
    child = top.getChild(i)
    print(f"  {child.getName()}")

# By iterator
for child in top:
    print(f"  {child.getName()}")

# By name
mesh_obj = top.getChildByName("mesh")
```

### Checking Object Types

```python
if obj.isPolyMesh():
    print("PolyMesh")
elif obj.isXform():
    print("Transform")
elif obj.isCamera():
    print("Camera")
elif obj.isCurves():
    print("Curves")
elif obj.isPoints():
    print("Points")
elif obj.isSubD():
    print("SubD")
```

## Reading Geometry

### PolyMesh

```python
if obj.isPolyMesh():
    # Get sample at frame 0
    sample = obj.getPolyMeshSample(0)
    
    # Vertex positions
    positions = sample.positions  # List of [x, y, z]
    print(f"Vertices: {len(positions)}")
    
    # Face topology
    face_counts = sample.faceCounts  # Vertices per face
    face_indices = sample.faceIndices  # Vertex indices
    print(f"Faces: {len(face_counts)}")
    
    # Optional data
    if sample.normals:
        print(f"Normals: {len(sample.normals)}")
    if sample.uvs:
        print(f"UVs: {len(sample.uvs)}")
    if sample.velocities:
        print(f"Velocities: {len(sample.velocities)}")
    if sample.selfBounds:
        min_pt, max_pt = sample.selfBounds
        print(f"Bounds: {min_pt} to {max_pt}")
```

### Transform (Xform)

```python
if obj.isXform():
    sample = obj.getXformSample(0)
    
    # 4x4 matrix
    matrix = sample.matrix
    
    # Decomposed components
    translation = sample.getTranslation()
    scale = sample.getScale()
    rotation = sample.getRotation()  # Euler XYZ degrees
    quaternion = sample.getRotationQuaternion()  # [x, y, z, w]
    
    # Inheritance
    if sample.inherits:
        print("Inherits parent transform")
```

### Camera

```python
if obj.isCamera():
    sample = obj.getCameraSample(0)
    
    print(f"Focal length: {sample.focalLength}mm")
    print(f"Aperture: {sample.horizontalAperture}x{sample.verticalAperture}")
    print(f"Near/Far: {sample.nearClippingPlane}/{sample.farClippingPlane}")
    print(f"F-stop: {sample.fStop}")
    print(f"Focus: {sample.focusDistance}")
    
    # Computed FOV
    print(f"H-FOV: {sample.getFovHorizontal():.1f} degrees")
```

### Curves

```python
if obj.isCurves():
    sample = obj.getCurvesSample(0)
    
    positions = sample.positions
    num_vertices = sample.numVertices  # Vertices per curve
    
    print(f"Curves: {len(num_vertices)}")
    print(f"Type: {sample.curveType}")
    print(f"Basis: {sample.basis}")
    print(f"Wrap: {sample.wrap}")
```

### Points

```python
if obj.isPoints():
    sample = obj.getPointsSample(0)
    
    positions = sample.positions
    ids = sample.ids
    
    print(f"Points: {len(positions)}")
    if sample.velocities:
        print(f"Has velocities")
    if sample.widths:
        print(f"Has widths")
```

## Animation

### Checking Sample Count

```python
num_samples = obj.getNumSamples()
print(f"Animation samples: {num_samples}")
```

### Reading All Frames

```python
for frame in range(obj.getNumSamples()):
    if obj.isPolyMesh():
        sample = obj.getPolyMeshSample(frame)
        # Process sample...
```

### Time Sampling

```python
# Get time sampling count
num_ts = archive.getNumTimeSamplings()

# Get time sampling info
for i in range(num_ts):
    ts = archive.getTimeSampling(i)
    if ts:
        print(f"TimeSampling {i}: {ts}")
```

## Convenience Methods

```python
# Quick position access for meshes
positions = obj.getPositions(0)

# Quick face data
face_counts = obj.getFaceCounts(0)
face_indices = obj.getFaceIndices(0)

# Quick matrix for xforms
matrix = obj.getMatrix(0)
```
