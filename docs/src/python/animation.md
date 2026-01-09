# Animation (Python)

## Time Sampling

Alembic stores animation using time samples - multiple data samples at different times.

### Types

1. **Identity** (index 0) - Static/constant data
2. **Uniform** - Regular intervals (e.g., 24 fps)
3. **Acyclic** - Arbitrary time values
4. **Cyclic** - Repeating patterns

## Reading Animation

### Checking for Animation

```python
num_samples = obj.getNumSamples()
if num_samples > 1:
    print(f"Animated: {num_samples} samples")
else:
    print("Static")
```

### Reading All Frames

```python
for frame in range(obj.getNumSamples()):
    if obj.isPolyMesh():
        sample = obj.getPolyMeshSample(frame)
        # Process positions, etc.
        
    elif obj.isXform():
        sample = obj.getXformSample(frame)
        matrix = sample.matrix
```

### Time Sampling Information

```python
# How many time samplings in the archive
num_ts = archive.getNumTimeSamplings()

for i in range(num_ts):
    ts = archive.getTimeSampling(i)
    if ts:
        print(f"TimeSampling {i}: {ts}")
```

## Writing Animation

### Setting Up Time Sampling

```python
# Uniform at 24 fps
ts_index = archive.addUniformTimeSampling(24.0, start_time=0.0)

# Uniform at 30 fps, starting at frame 100
ts_index = archive.addUniformTimeSampling(30.0, start_time=100/30.0)

# Acyclic - arbitrary times
times = [0.0, 0.5, 1.0, 2.0, 5.0]
ts_index = archive.addAcyclicTimeSampling(times)

# Cyclic - repeating pattern
ts_index = archive.addCyclicTimeSampling(
    time_per_cycle=1.0,
    times=[0.0, 0.25, 0.5, 0.75]
)
```

### Animated Transform

```python
import math

def write_spinning_cube(output_path, fps=24, duration=2.0):
    """Write a cube that spins for 2 seconds."""
    
    archive = OArchive.create(output_path)
    archive.setAppName("Animation Example")
    archive.setDccFps(fps)
    
    # Set up time sampling
    ts_index = archive.addUniformTimeSampling(fps, 0.0)
    
    # Create transform
    xform = alembic_rs.Abc.OXform("spinner")
    
    num_frames = int(fps * duration)
    for frame in range(num_frames):
        # Full rotation over duration
        t = frame / num_frames
        angle = t * 2 * math.pi
        
        # Rotation matrix around Y
        cos_a = math.cos(angle)
        sin_a = math.sin(angle)
        
        matrix = [
            [cos_a, 0.0, sin_a, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [-sin_a, 0.0, cos_a, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ]
        
        xform.addMatrixSample(matrix, inherits=True)
    
    # Add static mesh under transform
    mesh = create_cube_mesh()
    xform.addPolyMesh(mesh)
    
    archive.writeXform(xform)
    archive.close()
```

### Animated Geometry

```python
def write_bouncing_ball(output_path, fps=24, num_frames=48):
    """Write a deforming bouncing ball."""
    
    archive = OArchive.create(output_path)
    ts_index = archive.addUniformTimeSampling(fps, 0.0)
    
    mesh = alembic_rs.Abc.OPolyMesh("ball")
    
    # Fixed topology
    face_counts, face_indices = create_sphere_topology()
    
    for frame in range(num_frames):
        t = frame / num_frames
        
        # Bounce position
        height = abs(math.sin(t * 2 * math.pi)) * 2.0
        
        # Squash when hitting ground
        squash = 1.0 - 0.3 * (1.0 - height / 2.0)
        
        # Generate deformed sphere
        positions = create_sphere_positions(
            radius=0.5,
            y_scale=squash,
            y_offset=height + 0.5
        )
        
        mesh.addSample(positions, face_counts, face_indices)
    
    archive.writePolyMesh(mesh)
    archive.close()
```

### Motion Blur with Velocities

```python
def write_with_velocities(mesh, positions_sequence):
    """Write mesh with velocities for motion blur."""
    
    face_counts, face_indices = get_topology()
    
    for frame in range(len(positions_sequence)):
        positions = positions_sequence[frame]
        
        # Compute velocities (displacement to next frame)
        if frame < len(positions_sequence) - 1:
            next_positions = positions_sequence[frame + 1]
            velocities = [
                [n[0] - p[0], n[1] - p[1], n[2] - p[2]]
                for p, n in zip(positions, next_positions)
            ]
        else:
            velocities = [[0, 0, 0]] * len(positions)
        
        mesh.addSample(
            positions, 
            face_counts, 
            face_indices,
            velocities=velocities  # Note: API may vary
        )
```

## Interpolation

For playback at arbitrary times, you may need to interpolate between samples.

### Linear Position Interpolation

```python
def get_positions_at_time(obj, time, fps=24):
    """Get interpolated positions at arbitrary time."""
    
    num_samples = obj.getNumSamples()
    if num_samples == 1:
        return obj.getPolyMeshSample(0).positions
    
    # Find bracketing frames
    frame_float = time * fps
    frame_low = int(frame_float)
    frame_high = min(frame_low + 1, num_samples - 1)
    t = frame_float - frame_low
    
    # Get samples
    sample_low = obj.getPolyMeshSample(frame_low)
    sample_high = obj.getPolyMeshSample(frame_high)
    
    # Interpolate
    positions = []
    for p0, p1 in zip(sample_low.positions, sample_high.positions):
        positions.append([
            p0[0] + (p1[0] - p0[0]) * t,
            p0[1] + (p1[1] - p0[1]) * t,
            p0[2] + (p1[2] - p0[2]) * t,
        ])
    
    return positions
```

### Transform Interpolation

```python
def interpolate_matrix(m0, m1, t):
    """Simple linear matrix interpolation."""
    # For production, use proper decomposition (TRS) and slerp for rotation
    return [
        [m0[i][j] + (m1[i][j] - m0[i][j]) * t for j in range(4)]
        for i in range(4)
    ]
```

## Performance Tips

1. **Constant topology** - If only positions change, Alembic optimizes storage
2. **Deduplication** - Identical samples are stored once
3. **Sequential access** - Read frames in order when possible
4. **Batch processing** - Load all needed frames at once if memory allows
