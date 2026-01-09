# Properties

Alembic stores data in a hierarchical property system. This chapter covers reading and writing properties.

## Property Types

| Type | Description |
|------|-------------|
| **Scalar** | Single value per sample |
| **Array** | Array of values per sample |
| **Compound** | Container for other properties |

## Reading Properties

### Accessing Properties

```rust
let props = object.properties();

// By index
for i in 0..props.num_properties() {
    if let Some(prop) = props.property(i) {
        println!("{}: {:?}", prop.name(), prop.header().pod_type);
    }
}

// By name
if let Some(prop) = props.property_by_name("customData") {
    // ...
}
```

### Reading Scalar Properties

```rust
use alembic::abc::ITypedScalarProperty;

if let Some(scalar) = prop.as_scalar() {
    // Get info
    println!("Samples: {}", scalar.num_samples());
    println!("Type: {:?}", scalar.data_type());
    
    // Read as specific type
    let typed: ITypedScalarProperty<f32> = scalar.typed()?;
    let value = typed.get(0)?;
    
    // Or read raw bytes
    let mut buf = [0u8; 4];
    scalar.read_sample(0, &mut buf)?;
}
```

### Reading Array Properties

```rust
use alembic::abc::ITypedArrayProperty;

if let Some(array) = prop.as_array() {
    // Get dimensions
    let dims = array.get_dimensions(0)?;
    println!("Array size: {:?}", dims);
    
    // Read as typed
    let typed: ITypedArrayProperty<[f32; 3]> = array.typed()?;
    let values = typed.get(0)?;
    
    for v in values {
        println!("  {:?}", v);
    }
}
```

### Reading Compound Properties

```rust
if let Some(compound) = prop.as_compound() {
    // Iterate children
    for i in 0..compound.num_properties() {
        if let Some(child) = compound.property(i) {
            println!("  {}", child.name());
        }
    }
    
    // Access by name
    if let Some(child) = compound.property_by_name("nested") {
        // ...
    }
}
```

## Data Types

### POD (Plain Old Data) Types

| Rust Type | Alembic Type |
|-----------|--------------|
| `bool` | BOOL |
| `u8` | UINT8 |
| `i8` | INT8 |
| `u16` | UINT16 |
| `i16` | INT16 |
| `u32` | UINT32 |
| `i32` | INT32 |
| `u64` | UINT64 |
| `i64` | INT64 |
| `f32` | FLOAT32 |
| `f64` | FLOAT64 |
| `String` | STRING |

### Geometric Types

| Rust Type | Alembic Type |
|-----------|--------------|
| `[f32; 2]` | VEC2F |
| `[f32; 3]` | VEC3F |
| `[f32; 4]` | VEC4F |
| `[f64; 2]` | VEC2D |
| `[f64; 3]` | VEC3D |
| `[i32; 2]` | VEC2I |
| `[i32; 3]` | VEC3I |
| `[[f32; 3]; 3]` | MAT33F |
| `[[f32; 4]; 4]` | MAT44F |

## Writing Properties

### Creating Properties

```rust
use alembic::ogawa::writer::OProperty;
use alembic::util::DataType;

// Scalar property
let mut scalar_prop = OProperty::scalar("temperature", DataType::FLOAT32);
scalar_prop.add_scalar_pod(&25.5f32);

// Array property
let mut array_prop = OProperty::array("colors", DataType::VEC3F);
let colors: Vec<[f32; 3]> = vec![[1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
array_prop.add_array_pod(&colors);

// String property
let mut string_prop = OProperty::scalar("name", DataType::STRING);
string_prop.add_scalar_sample(b"MyObject\0");
```

### Adding to Objects

```rust
let mut obj = OObject::new("custom");
obj.add_property(scalar_prop);
obj.add_property(array_prop);
```

### Compound Properties

```rust
// Create compound
let mut compound = OProperty::compound("userProperties");

// Add children
compound.add_child(OProperty::scalar("id", DataType::INT32));
compound.add_child(OProperty::array("weights", DataType::FLOAT32));

obj.add_property(compound);
```

## Time-Varying Properties

```rust
// Create property with time sampling
let mut prop = OProperty::scalar("animated", DataType::FLOAT32);
prop.time_sampling_index = ts_index;

// Add samples for each frame
for frame in 0..100 {
    let value = frame as f32 * 0.1;
    prop.add_scalar_pod(&value);
}
```
