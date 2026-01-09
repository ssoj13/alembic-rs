# Writing Archives

## Creating an Archive

```rust
use alembic::ogawa::writer::OArchive;

let mut archive = OArchive::create("output.abc")?;

// Set metadata
archive.set_app_name("My Application");
archive.set_description("Scene export");
archive.set_application_writer("alembic-rs 0.1.0");

// Configure compression (0-9, -1 = disabled)
archive.set_compression_hint(6);

// Enable deduplication
archive.set_dedup_enabled(true);
```

## Writing Geometry

### PolyMesh

```rust
use alembic::ogawa::writer::{OPolyMesh, OPolyMeshSample};
use glam::Vec3;

let mut mesh = OPolyMesh::new("cube");

// Create sample
let positions = vec![
    Vec3::new(-1.0, -1.0, -1.0),
    Vec3::new( 1.0, -1.0, -1.0),
    // ... more vertices
];

let face_counts = vec![4, 4, 4, 4, 4, 4];  // 6 quads
let face_indices = vec![0, 1, 2, 3, /* ... */];

let mut sample = OPolyMeshSample::new(positions, face_counts, face_indices);

// Optional: add normals
sample.normals = Some(vec![/* ... */]);

// Optional: add UVs
sample.uvs = Some(vec![/* ... */]);

mesh.add_sample(&sample);

// Write to archive
archive.write_archive(&mesh.build())?;
```

### Transform (Xform)

```rust
use alembic::ogawa::writer::{OXform, OXformSample};
use glam::Mat4;

let mut xform = OXform::new("transform");

// Identity
xform.add_sample(OXformSample::identity());

// From matrix
let matrix = Mat4::from_translation(glam::Vec3::new(1.0, 2.0, 3.0));
xform.add_sample(OXformSample::from_matrix(matrix, true));

// Add child mesh
xform.add_child(mesh.build());

archive.write_archive(&xform.build())?;
```

### Curves

```rust
use alembic::ogawa::writer::{OCurves, OCurvesSample};
use alembic::geom::{CurveType, CurvePeriodicity};

let mut curves = OCurves::new("hair");

let positions = vec![/* curve points */];
let num_vertices = vec![10, 10, 10];  // 3 curves with 10 points each

let sample = OCurvesSample::new(positions, num_vertices)
    .with_curve_type(CurveType::Cubic)
    .with_wrap(CurvePeriodicity::NonPeriodic);

curves.add_sample(&sample);
```

### Points

```rust
use alembic::ogawa::writer::{OPoints, OPointsSample};

let mut points = OPoints::new("particles");

let positions = vec![/* point positions */];
let ids: Vec<u64> = (0..positions.len() as u64).collect();

let mut sample = OPointsSample::new(positions, ids);
sample.velocities = Some(vec![/* velocities */]);
sample.widths = Some(vec![/* radii */]);

points.add_sample(&sample);
```

## Building Hierarchies

```rust
use alembic::ogawa::writer::OObject;

// Create root
let mut root = OObject::new("scene");

// Add transform with mesh
let mut xform = OXform::new("group");
xform.add_child(mesh.build());
root.add_child(xform.build());

// Add another mesh at root level
root.add_child(another_mesh.build());

archive.write_archive(&root)?;
```

## Time Sampling

```rust
use alembic::core::TimeSampling;

// Uniform (e.g., 24 fps)
let ts_index = archive.add_time_sampling(
    TimeSampling::uniform(1.0 / 24.0, 0.0)
);

// Acyclic (arbitrary times)
let ts_index = archive.add_time_sampling(
    TimeSampling::acyclic(vec![0.0, 0.5, 1.0, 2.0])
);

// Use on a mesh
let mut mesh = OPolyMesh::new("animated");
mesh.set_time_sampling_index(ts_index);

// Add samples for each frame
for frame in 0..48 {
    mesh.add_sample(&samples[frame]);
}
```

## Finalizing

```rust
// Write and close
archive.write_archive(&root)?;
archive.close()?;
```
