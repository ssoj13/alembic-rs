# Working with Geometry (Python)

## PolyMesh

### Reading

```python
if obj.isPolyMesh():
    sample = obj.getPolyMeshSample(0)
    
    # Core data
    positions = sample.positions      # [[x, y, z], ...]
    face_counts = sample.faceCounts   # [4, 4, 3, ...]
    face_indices = sample.faceIndices # [0, 1, 2, 3, ...]
    
    # Statistics
    num_verts = sample.getNumVertices()
    num_faces = sample.getNumFaces()
    
    # Optional attributes
    normals = sample.normals      # [[nx, ny, nz], ...] or None
    uvs = sample.uvs              # [[u, v], ...] or None
    velocities = sample.velocities
    bounds = sample.selfBounds    # (min, max) or None
```

### Writing

```python
mesh = alembic_rs.Abc.OPolyMesh("mesh")

# Basic sample
mesh.addSample(positions, face_counts, face_indices)

# With optional data
mesh.addSample(
    positions,
    face_counts,
    face_indices,
    normals=normals,
    uvs=uvs
)
```

## SubD (Subdivision Surfaces)

### Reading

```python
if obj.isSubD():
    sample = obj.getSubDSample(0)
    
    # Base mesh
    positions = sample.positions
    face_counts = sample.faceCounts
    face_indices = sample.faceIndices
    
    # Subdivision scheme
    scheme = sample.scheme  # "catmullClark", "loop", "bilinear"
    
    # Creases
    crease_indices = sample.creaseIndices
    crease_lengths = sample.creaseLengths
    crease_sharpnesses = sample.creaseSharpnesses
    
    # Corners
    corner_indices = sample.cornerIndices
    corner_sharpnesses = sample.cornerSharpnesses
    
    # Holes (faces to skip)
    holes = sample.holes
```

### Writing

```python
subd = alembic_rs.Abc.OSubD("subdivision")

subd.addSample(
    positions,
    face_counts,
    face_indices,
    scheme="catmullClark",
    crease_indices=[0, 1, 1, 2],  # Edge vertex pairs
    crease_lengths=[2, 2],        # Vertices per crease
    crease_sharpnesses=[2.0, 3.0],
    corner_indices=[5],
    corner_sharpnesses=[2.5],
    holes=[3],  # Face indices to treat as holes
)
```

## Curves

### Reading

```python
if obj.isCurves():
    sample = obj.getCurvesSample(0)
    
    # All points for all curves
    positions = sample.positions
    
    # Points per curve
    num_vertices = sample.numVertices
    
    # Curve properties
    curve_type = sample.curveType  # "Linear", "Cubic"
    basis = sample.basis           # "NoBasis", "Bezier", etc.
    wrap = sample.wrap             # "NonPeriodic", "Periodic"
    
    # Optional
    widths = sample.widths
    orders = sample.orders
    knots = sample.knots
    
    # Number of curves
    num_curves = sample.getNumCurves()
```

### Extracting Individual Curves

```python
def extract_curves(sample):
    """Extract individual curves from concatenated data."""
    curves = []
    offset = 0
    
    for count in sample.numVertices:
        curve_points = sample.positions[offset:offset + count]
        curves.append(curve_points)
        offset += count
    
    return curves
```

### Writing

```python
curves = alembic_rs.Abc.OCurves("curves")

curves.addSample(
    positions,          # All points concatenated
    num_vertices,       # Points per curve
    curve_type="cubic",
    basis="bspline",
    wrap="nonperiodic",
    widths=widths,
    orders=orders,      # For NURBS curves
    knots=knots,        # For NURBS curves
)
```

## Points

### Reading

```python
if obj.isPoints():
    sample = obj.getPointsSample(0)
    
    positions = sample.positions
    ids = sample.ids  # For tracking across frames
    velocities = sample.velocities
    widths = sample.widths
```

### Writing

```python
points = alembic_rs.Abc.OPoints("particles")

points.addSample(
    positions,
    ids,               # Unique per-point IDs
    velocities=velocities,
    widths=widths,
)
```

## Camera

### Reading

```python
if obj.isCamera():
    sample = obj.getCameraSample(0)
    
    # Lens
    focal = sample.focalLength           # mm
    squeeze = sample.lensSqueezeRatio
    
    # Film/Sensor
    h_aperture = sample.horizontalAperture
    v_aperture = sample.verticalAperture
    h_offset = sample.horizontalFilmOffset
    v_offset = sample.verticalFilmOffset
    
    # Clipping
    near = sample.nearClippingPlane
    far = sample.farClippingPlane
    
    # Depth of Field
    fstop = sample.fStop
    focus = sample.focusDistance
    
    # Shutter
    shutter_open = sample.shutterOpen
    shutter_close = sample.shutterClose
    
    # Computed values
    h_fov = sample.getFovHorizontal()
    v_fov = sample.getFovVertical()
```

### Writing

```python
camera = alembic_rs.Abc.OCamera("camera")

camera.addSample(
    focal_length=50.0,
    horizontal_aperture=36.0,
    vertical_aperture=24.0,
    near_clipping_plane=0.1,
    far_clipping_plane=10000.0,
    f_stop=2.8,
    focus_distance=2.0,
)
```

## FaceSet

FaceSets group faces within a mesh for material assignment.

### Reading

```python
# List FaceSet names
names = mesh_obj.getFaceSetNames()

# Get specific FaceSet
if names:
    faceset = mesh_obj.getFaceSet(names[0])
    sample = faceset.getSample(0)
    
    face_indices = sample.faces  # Which faces belong to this set
    print(f"FaceSet '{faceset.getName()}' has {len(face_indices)} faces")
```

### Writing

```python
faceset = alembic_rs.Abc.OFaceSet("material_group")
faceset.addSample([0, 1, 2, 3])  # Face indices in this set

# Add to mesh object
mesh_obj.addFaceSet(faceset)
```

## Visibility

### Reading

```python
# Get visibility at frame
visibility = obj.getVisibility(0)

if visibility.isDeferred():
    print("Inherit from parent")
elif visibility.isHidden():
    print("Hidden")
elif visibility.isVisible():
    print("Visible")

# Quick check
if obj.isVisible(0):
    # Render this object
    pass
```
