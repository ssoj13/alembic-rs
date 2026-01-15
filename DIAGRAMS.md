# Alembic-RS Architecture Diagrams

## 1. High-Level Module Structure

```mermaid
graph TB
    subgraph "Public API"
        ABC[abc module<br/>IArchive, OArchive<br/>IObject, IProperty]
        GEOM[geom module<br/>IPolyMesh, IXform<br/>ICurves, IPoints...]
        MAT[material module<br/>IMaterial, Schema]
        COLL[collection module<br/>ICollections]
    end

    subgraph "Core Layer"
        CORE[core module<br/>Traits: Reader/Writer<br/>TimeSampling, DataType]
        UTIL[util module<br/>SpookyHash, Murmur3<br/>POD types]
    end

    subgraph "Backend"
        OGAWA[ogawa module<br/>Binary format<br/>Read/Write]
    end

    subgraph "Viewer (Optional)"
        VIEW[viewer module<br/>wgpu renderer]
        SHADER[Standard Surface<br/>MaterialX port]
    end

    ABC --> CORE
    GEOM --> ABC
    MAT --> ABC
    COLL --> ABC
    CORE --> OGAWA
    CORE --> UTIL
    VIEW --> GEOM
    VIEW --> SHADER
```

## 2. Ogawa File Format Structure

```mermaid
graph TD
    subgraph "Ogawa Archive (.abc)"
        HEADER[Header<br/>Magic: 0xff00aa55<br/>Version: 10709<br/>Frozen flag]
        ROOT[Root Group]
        
        subgraph "Object Hierarchy"
            OBJ1[Object: /root]
            OBJ2[Object: /root/xform]
            OBJ3[Object: /root/xform/mesh]
        end
        
        subgraph "Data Groups"
            META[MetaData Group<br/>Schema info]
            PROPS[Properties Group<br/>Compound/Scalar/Array]
            SAMPLES[Sample Data<br/>with deduplication]
        end
    end

    HEADER --> ROOT
    ROOT --> OBJ1
    OBJ1 --> OBJ2
    OBJ2 --> OBJ3
    OBJ1 --> META
    OBJ1 --> PROPS
    PROPS --> SAMPLES
```

## 3. Reading Pipeline (IArchive)

```mermaid
sequenceDiagram
    participant App as Application
    participant IA as IArchive
    participant OR as OgawaReader
    participant File as .abc File

    App->>IA: IArchive::open(path)
    IA->>OR: OgawaReader::new()
    OR->>File: Read header
    File-->>OR: Magic + Version
    OR->>File: Read root group
    File-->>OR: Group data
    OR-->>IA: Archive ready
    IA-->>App: IArchive instance

    App->>IA: top()
    IA-->>App: IObject (root)
    
    App->>IA: child(name)
    IA->>OR: Read child group
    OR->>File: Seek + Read
    File-->>OR: Child data
    OR-->>IA: Child object
    IA-->>App: IObject (child)
```

## 4. Writing Pipeline (OArchive)

```mermaid
sequenceDiagram
    participant App as Application
    participant OA as OArchive
    participant OW as OgawaWriter
    participant DD as Deduplicator
    participant File as .abc File

    App->>OA: OArchive::new(path)
    OA->>OW: OgawaWriter::new()
    OW->>File: Write header
    OA-->>App: OArchive ready

    App->>OA: add_child(name, schema)
    OA->>OW: Create object group
    OA-->>App: ObjectWriter

    App->>OA: write_sample(data)
    OA->>DD: Hash sample (SpookyHash)
    DD-->>OA: Hash value
    alt Sample exists
        OA->>OA: Reference existing
    else New sample
        OA->>OW: Write sample data
        OW->>File: Write bytes
    end

    App->>OA: close()
    OA->>OW: Finalize groups
    OW->>File: Write index + close
```

## 5. Geometry Schema Hierarchy

```mermaid
classDiagram
    class IGeomBase {
        +get_self_bounds()
        +get_child_bounds()
        +get_schema()
    }
    
    class IPolyMesh {
        +positions: ArrayProperty~P3f~
        +face_indices: ArrayProperty~i32~
        +face_counts: ArrayProperty~i32~
        +normals: ArrayProperty~N3f~
        +uvs: ArrayProperty~V2f~
    }
    
    class IXform {
        +ops: Vec~XformOp~
        +inherits: bool
        +get_matrix()
    }
    
    class ICurves {
        +positions: ArrayProperty~P3f~
        +num_vertices: ArrayProperty~i32~
        +type: CurveType
        +wrap: CurvePeriodicity
    }
    
    class IPoints {
        +positions: ArrayProperty~P3f~
        +ids: ArrayProperty~u64~
        +velocities: ArrayProperty~V3f~
    }
    
    class ISubD {
        +positions: ArrayProperty~P3f~
        +face_indices: ArrayProperty~i32~
        +face_counts: ArrayProperty~i32~
        +crease_indices: ArrayProperty~i32~
        +crease_sharpnesses: ArrayProperty~f32~
    }

    IGeomBase <|-- IPolyMesh
    IGeomBase <|-- IXform
    IGeomBase <|-- ICurves
    IGeomBase <|-- IPoints
    IGeomBase <|-- ISubD
```

## 6. Viewer Render Pipeline

```mermaid
graph LR
    subgraph "Input"
        ABC_FILE[.abc File]
        HDR[Environment HDR]
    end

    subgraph "Conversion"
        MESH_CONV[MeshConverter<br/>cache + parallel]
        NORM_CALC[SmoothNormals<br/>angle threshold]
    end

    subgraph "GPU Resources"
        VBO[Vertex Buffers<br/>pos, norm, uv]
        IBO[Index Buffers]
        TEX[Textures<br/>env, shadow]
        UBO[Uniform Buffers<br/>camera, light]
    end

    subgraph "Render Passes"
        SHADOW[Shadow Pass<br/>depth only]
        SOLID[Solid Pass<br/>Standard Surface]
        WIRE[Wireframe Pass]
        SKY[Skybox Pass]
    end

    subgraph "Output"
        FB[Framebuffer]
        SCREEN[Screen]
    end

    ABC_FILE --> MESH_CONV
    MESH_CONV --> NORM_CALC
    NORM_CALC --> VBO
    MESH_CONV --> IBO
    HDR --> TEX

    VBO --> SHADOW
    VBO --> SOLID
    VBO --> WIRE
    IBO --> SHADOW
    IBO --> SOLID
    IBO --> WIRE
    TEX --> SOLID
    TEX --> SKY
    UBO --> SHADOW
    UBO --> SOLID
    UBO --> WIRE

    SHADOW --> FB
    SKY --> FB
    SOLID --> FB
    WIRE --> FB
    FB --> SCREEN
```

## 7. TimeSampling System

```mermaid
graph TB
    subgraph "TimeSampling Types"
        UNIFORM[Uniform<br/>start + interval]
        CYCLIC[Cyclic<br/>repeating pattern]
        ACYCLIC[Acyclic<br/>arbitrary times]
    end

    subgraph "Sample Storage"
        TS_REG[TimeSampling Registry<br/>shared across archive]
        SAMPLE_IDX[Sample Index<br/>per property]
    end

    subgraph "Interpolation"
        FLOOR[Floor Sample]
        CEIL[Ceil Sample]
        LERP[Linear Interpolation]
    end

    UNIFORM --> TS_REG
    CYCLIC --> TS_REG
    ACYCLIC --> TS_REG
    
    TS_REG --> SAMPLE_IDX
    SAMPLE_IDX --> FLOOR
    SAMPLE_IDX --> CEIL
    FLOOR --> LERP
    CEIL --> LERP
```

## 8. Property Type Hierarchy

```mermaid
classDiagram
    class Property {
        <<interface>>
        +name(): String
        +metadata(): MetaData
        +data_type(): DataType
    }
    
    class CompoundProperty {
        +children: Vec~Property~
        +get_child(name)
        +num_children()
    }
    
    class ScalarProperty {
        +num_samples(): usize
        +get_sample(index)
        +is_constant(): bool
    }
    
    class ArrayProperty {
        +num_samples(): usize
        +get_sample(index)
        +get_dimensions()
    }

    Property <|-- CompoundProperty
    Property <|-- ScalarProperty
    Property <|-- ArrayProperty
    
    CompoundProperty o-- Property : contains
```

## 9. Parity Status Overview

```mermaid
pie title Implementation Status
    "Fully Implemented" : 60
    "Partial (Read Only)" : 25
    "Stub/Planned" : 10
    "Not Planned" : 5
```

## 10. Module Dependencies

```mermaid
graph BT
    UTIL[util]
    OGAWA[ogawa]
    CORE[core]
    ABC[abc]
    GEOM[geom]
    MAT[material]
    COLL[collection]
    VIEW[viewer]
    STDSRF[standard-surface]
    
    OGAWA --> UTIL
    CORE --> UTIL
    CORE --> OGAWA
    ABC --> CORE
    GEOM --> ABC
    MAT --> ABC
    COLL --> ABC
    VIEW --> GEOM
    VIEW --> STDSRF
    STDSRF --> UTIL
```

## Legend

| Symbol | Meaning |
|--------|---------|
| Solid Arrow | Direct dependency |
| Dashed Arrow | Optional dependency |
| Box | Module/Component |
| Diamond | Decision point |
| Cylinder | Data storage |
