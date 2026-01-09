# Custom Properties

Alembic supports custom properties (arbitrary attributes) that can be attached to any object. This allows storing application-specific metadata alongside geometry.

## Property Types

### Scalar Properties

Single values per sample:

```rust
use alembic_rs::ogawa::writer::OProperty;

// Create scalar properties
let int_prop = OProperty::scalar_i32("my_int", vec![42]);
let float_prop = OProperty::scalar_f32("my_float", vec![3.14]);
let string_prop = OProperty::scalar_string("my_string", vec!["hello".to_string()]);
```

### Array Properties

Arrays of values per sample:

```rust
// Array of floats
let weights = OProperty::array_f32("weights", vec![0.1, 0.5, 0.3, 0.1]);

// Array of integers
let ids = OProperty::array_i32("vertex_ids", vec![0, 1, 2, 3, 4]);

// Array of vectors (as flat f32 array)
let colors = OProperty::array_f32("vertex_colors", vec![
    1.0, 0.0, 0.0,  // red
    0.0, 1.0, 0.0,  // green
    0.0, 0.0, 1.0,  // blue
]);
```

## Adding Properties to Objects

### To PolyMesh

```rust
use alembic_rs::ogawa::writer::{OPolyMesh, OProperty};

let mut mesh = OPolyMesh::new("my_mesh");

// Add geometry sample
mesh.add_sample(&positions, &face_counts, &face_indices);

// Add custom properties
mesh.add_property(OProperty::scalar_string("material_name", vec!["metal".to_string()]));
mesh.add_property(OProperty::array_f32("vertex_weights", weights));
```

### To Xform

```rust
use alembic_rs::ogawa::writer::{OXform, OProperty};

let mut xform = OXform::new("my_xform");
xform.add_translation_sample(0.0, 1.0, 0.0);

// Add metadata
xform.add_property(OProperty::scalar_string("node_type", vec!["group".to_string()]));
xform.add_property(OProperty::scalar_i32("layer_id", vec![5]));
```

## Animated Properties

Properties can be animated by providing multiple samples:

```rust
// Animated float property
let animated_weight = OProperty::scalar_f32_animated(
    "blend_weight",
    vec![0.0, 0.5, 1.0],  // values at each time sample
);

// Animated array property
let animated_colors = OProperty::array_f32_animated(
    "vertex_colors",
    vec![
        vec![1.0, 0.0, 0.0, 0.0, 1.0, 0.0],  // frame 0
        vec![0.5, 0.5, 0.0, 0.5, 0.5, 0.5],  // frame 1
        vec![0.0, 1.0, 0.0, 1.0, 0.0, 0.0],  // frame 2
    ],
);
```

## Reading Custom Properties

```rust
use alembic_rs::ogawa::reader::IArchive;

let archive = IArchive::open("file.abc")?;
let root = archive.root();

for child in root.children() {
    // Get user properties
    let props = child.user_properties();
    
    for prop in props {
        println!("Property: {} (type: {:?})", prop.name(), prop.data_type());
        
        // Read based on type
        match prop.data_type() {
            DataType::Int32 => {
                if let Some(values) = prop.get_i32_samples() {
                    println!("  Values: {:?}", values);
                }
            }
            DataType::Float32 => {
                if let Some(values) = prop.get_f32_samples() {
                    println!("  Values: {:?}", values);
                }
            }
            DataType::String => {
                if let Some(values) = prop.get_string_samples() {
                    println!("  Values: {:?}", values);
                }
            }
            _ => {}
        }
    }
}
```

## Python API

```python
import alembic_rs

# Writing custom properties
mesh = alembic_rs.Abc.OPolyMesh("mesh")
mesh.addSample(positions, face_counts, face_indices)
mesh.addPropertyFloat("weight", 0.5)
mesh.addPropertyInt("id", 42)
mesh.addPropertyString("name", "my_object")

# Reading custom properties
archive = alembic_rs.IArchive("file.abc")
for obj in archive.root().children():
    props = obj.properties()
    for name, value in props.items():
        print(f"{name}: {value}")
```

## Best Practices

1. **Use consistent naming**: Establish naming conventions for custom properties
2. **Document property schemas**: Keep track of what properties your pipeline uses
3. **Consider scope**: Use geom properties for per-vertex data, user properties for metadata
4. **Type safety**: Always check property types before reading
5. **Versioning**: Include version info if property schemas may change
