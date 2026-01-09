# Animation

Alembic supports animation through time sampling - storing multiple samples at different times.

## Time Sampling Concepts

### Types of Time Sampling

1. **Identity** (index 0) - Static data, single sample
2. **Uniform** - Regular intervals (e.g., 24 fps)
3. **Cyclic** - Repeating pattern
4. **Acyclic** - Arbitrary time values

### Time Sampling vs Frame Numbers

Alembic stores actual time values, not frame numbers:
- Frame 0 at 24fps = 0.0 seconds
- Frame 1 at 24fps = 0.04167 seconds (1/24)
- Frame 24 = 1.0 seconds

## Reading Animated Data

### Checking for Animation

```rust
let mesh = IPolyMesh::new(&object)?;

let num_samples = mesh.num_samples();
if num_samples > 1 {
    println!("Animated mesh with {} samples", num_samples);
}
```

### Getting Time Sampling

```rust
// Archive-level time samplings
for i in 0..archive.num_time_samplings() {
    if let Some(ts) = archive.time_sampling(i) {
        println!("TimeSampling {}: {:?}", i, ts);
    }
}
```

### Reading at Specific Times

```rust
// By sample index
let sample = mesh.get_sample(frame_index)?;

// Find sample for a time
let time = 1.5;  // seconds
let (index, lerp_factor) = mesh.get_sample_index_for_time(time)?;

let sample = mesh.get_sample(index)?;
// lerp_factor can be used to interpolate between samples
```

### Interpolating Transforms

```rust
fn interpolate_xform(xform: &IXform, time: f64) -> Mat4 {
    let num = xform.num_samples();
    if num == 1 {
        return xform.get_sample(0).unwrap().matrix();
    }
    
    // Find bracketing samples
    let (idx, t) = xform.get_sample_index_for_time(time).unwrap();
    
    if t < 0.0001 || idx + 1 >= num {
        return xform.get_sample(idx).unwrap().matrix();
    }
    
    // Interpolate between samples
    let s0 = xform.get_sample(idx).unwrap().matrix();
    let s1 = xform.get_sample(idx + 1).unwrap().matrix();
    
    // Linear interpolation (for simple cases)
    // For production use, decompose and interpolate components
    lerp_mat4(s0, s1, t as f32)
}
```

## Writing Animated Data

### Setting Up Time Sampling

```rust
use alembic::core::TimeSampling;

let mut archive = OArchive::create("animated.abc")?;

// Uniform sampling at 24 fps
let ts = TimeSampling::uniform(1.0 / 24.0, 0.0);
let ts_index = archive.add_time_sampling(ts);

// Or acyclic for irregular times
let times = vec![0.0, 0.5, 1.0, 1.5, 3.0];
let ts = TimeSampling::acyclic(times);
let ts_index = archive.add_time_sampling(ts);
```

### Writing Animated Transforms

```rust
let mut xform = OXform::new("animated_xform");

// 48 frames at 24 fps = 2 seconds
for frame in 0..48 {
    let angle = (frame as f32 / 48.0) * std::f32::consts::TAU;
    
    let matrix = Mat4::from_rotation_y(angle);
    xform.add_sample(OXformSample::from_matrix(matrix, true));
}
```

### Writing Animated Geometry

```rust
let mut mesh = OPolyMesh::new("deforming_mesh");

// Fixed topology
let face_counts = vec![4; 6];  // Cube: 6 quads
let face_indices = vec![/* ... */];

for frame in 0..100 {
    // Deform positions
    let t = frame as f32 / 100.0;
    let positions = generate_deformed_cube(t);
    
    let mut sample = OPolyMeshSample::new(
        positions, 
        face_counts.clone(), 
        face_indices.clone()
    );
    
    // Velocities help with motion blur
    sample.velocities = Some(compute_velocities(frame));
    
    mesh.add_sample(&sample);
}
```

### Velocity for Motion Blur

```rust
// Velocities represent movement per frame
let velocities: Vec<Vec3> = positions
    .iter()
    .zip(previous_positions.iter())
    .map(|(curr, prev)| *curr - *prev)
    .collect();

sample.velocities = Some(velocities);
```

## Performance Tips

1. **Constant Topology**: If only positions change, topology is stored once
2. **Deduplication**: Identical samples are automatically deduplicated
3. **Read Sequentially**: Sequential sample access is faster than random
4. **Cache Samples**: Store frequently accessed samples in memory
