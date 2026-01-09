# Materials and Face Sets

Alembic provides mechanisms for material assignment through face sets and material bindings. This chapter covers how to work with materials in alembic-rs.

## Face Sets

Face sets group faces by material assignment. Each face set has a name (typically the material name) and a list of face indices.

### Writing Face Sets

```rust
use alembic_rs::ogawa::writer::{OPolyMesh, OFaceSet};

let mut mesh = OPolyMesh::new("mesh_with_materials");

// Add geometry
mesh.add_sample(&positions, &face_counts, &face_indices);

// Create face sets for material assignment
// Faces 0-3 use "metal" material
mesh.add_face_set(OFaceSet::new("metal", vec![0, 1, 2, 3]));

// Faces 4-7 use "wood" material  
mesh.add_face_set(OFaceSet::new("wood", vec![4, 5, 6, 7]));

// Remaining faces use "default" material
mesh.add_face_set(OFaceSet::new("default", vec![8, 9, 10, 11]));
```

### Reading Face Sets

```rust
use alembic_rs::ogawa::reader::IArchive;

let archive = IArchive::open("model.abc")?;

fn process_mesh(mesh: &IPolyMesh) {
    // Get face sets
    let face_sets = mesh.face_sets();
    
    for fs in face_sets {
        println!("Material: {}", fs.name());
        println!("  Faces: {:?}", fs.faces());
    }
}
```

## Material Targets

Material targets allow referencing external material definitions:

```rust
use alembic_rs::ogawa::writer::{OPolyMesh, OProperty};

let mut mesh = OPolyMesh::new("mesh");
mesh.add_sample(&positions, &face_counts, &face_indices);

// Add material target reference
mesh.add_property(OProperty::scalar_string(
    "material_target",
    vec!["/materials/chrome".to_string()]
));
```

## Material Schema

For full material information, use the Material schema:

```rust
// Material as a separate object in hierarchy
let mut materials_xform = OXform::new("materials");

// Material definition with properties
let mut chrome = OXform::new("chrome");
chrome.add_property(OProperty::scalar_string("shader", vec!["pbr_metallic".to_string()]));
chrome.add_property(OProperty::array_f32("base_color", vec![0.8, 0.8, 0.85]));
chrome.add_property(OProperty::scalar_f32("metallic", vec![1.0]));
chrome.add_property(OProperty::scalar_f32("roughness", vec![0.2]));

materials_xform.add_child(chrome);
```

## Integration with DCCs

### Maya Convention

```rust
// Maya-style shading group reference
mesh.add_property(OProperty::scalar_string(
    "ABC_shop_materialpath",
    vec!["metalSG".to_string()]
));
```

### Houdini Convention

```rust
// Houdini-style material path
mesh.add_property(OProperty::scalar_string(
    "shop_materialpath", 
    vec!["/mat/chrome".to_string()]
));
```

## Python API

```python
import alembic_rs

# Create mesh with face sets
mesh = alembic_rs.Abc.OPolyMesh("mesh")
mesh.addSample(positions, face_counts, face_indices)

# Add face sets
mesh.addFaceSet("metal", [0, 1, 2, 3])
mesh.addFaceSet("wood", [4, 5, 6, 7])

# Add material reference
mesh.addPropertyString("material_target", "/materials/chrome")
```

## Best Practices

1. **Consistent naming**: Use same material names across your pipeline
2. **Face coverage**: Ensure all faces are assigned to a face set
3. **No overlaps**: Each face should belong to only one face set
4. **External references**: Store material definitions separately when possible
5. **DCC compatibility**: Follow conventions of your target application
