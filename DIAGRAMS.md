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

## 5. Writer Parity Order (Ogawa)

```mermaid
sequenceDiagram
    participant App as Application
    participant OA as OArchive
    participant OW as OgawaWriter
    participant Obj as OObject
    participant Props as OProperty

    App->>OA: write_archive(root)
    OA->>OW: write version + library version
    OW->>Obj: write children first
    Obj->>Props: write sample data
    Props-->>Obj: property groups (reverse order)
    Obj->>OW: object headers (data hash + child hash)
    Obj->>OW: property headers
    OW->>OW: finalize object group
    OW->>OW: write archive metadata
    OW->>OW: write time samplings
    OW->>OW: write indexed metadata
    OW->>OW: write root group + frozen flag
```

Note: time sampling tables use maxSamples = 1 for constant properties; archive metadata always includes _ai_AlembicVersion.

## 6. Geometry Schema Hierarchy

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

## 7. Viewer Render Pipeline

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
        OPAQUE[Opaque Pass<br/>Standard Surface]
        TRANSPARENT[Transparent Pass<br/>sorted back-to-front]
        LINES[Lines/Points Pass]
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
    VBO --> OPAQUE
    VBO --> TRANSPARENT
    VBO --> LINES
    IBO --> SHADOW
    IBO --> OPAQUE
    IBO --> TRANSPARENT
    IBO --> LINES
    TEX --> OPAQUE
    TEX --> TRANSPARENT
    TEX --> SKY
    UBO --> SHADOW
    UBO --> OPAQUE
    UBO --> TRANSPARENT
    UBO --> LINES

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

## 11. Output Schema Architecture (Fixed)

```mermaid
graph LR
    subgraph "geom/mod.rs - Re-exports"
        GOM[OPolyMesh<br/>re-export]
        GOX[OXform<br/>re-export]
        GOC[OCurves<br/>re-export]
    end

    subgraph "ogawa/writer.rs - Implementations"
        WOM[OPolyMesh<br/>object + geom_compound]
        WOX[OXform<br/>object + samples]
        WOC[OCurves<br/>object + geom_compound]
    end

    subgraph "User API"
        U[use geom::OPolyMesh]
    end

    GOM -->|"re-exports"| WOM
    GOX -->|"re-exports"| WOX
    GOC -->|"re-exports"| WOC
    U -->|"uses"| GOM

    style GOM fill:#9f9
    style GOX fill:#9f9
    style GOC fill:#9f9
    style WOM fill:#9f9
    style WOX fill:#9f9
    style WOC fill:#9f9
```

**Status**: ✅ FIXED - geom/mod.rs now re-exports types from ogawa/writer.rs.

## 12. Code Deduplication (Fixed)

```mermaid
flowchart TB
    subgraph "Schema Writers"
        OPM["OPolyMesh<br/>uses get_or_create_array_with_ts()"]
        OCR["OCurves"]
        OPT["OPoints"]
        OSD["OSubD"]
        OFS["OFaceSet"]
    end

    subgraph "Shared Helper - OProperty"
        IMPL["get_or_create_array_child()<br/>get_or_create_scalar_child()"]
    end

    OCR -->|"uses"| IMPL
    OPT -->|"uses"| IMPL
    OSD -->|"uses"| IMPL
    OFS -->|"uses"| IMPL
    OPM -.->|"special version"| OPM

    style IMPL fill:#9f9
```

**Status**: ✅ FIXED - Shared helpers added to OProperty. ~60 lines of duplicate code removed.

## 13. Read vs Write API (Fixed)

```mermaid
flowchart LR
    subgraph "READ PATH"
        R1[abc::IArchive]
        R2[abc::IObject]
        R3[geom::IPolyMesh]
        R4[ogawa::reader]
    end

    subgraph "WRITE PATH"
        W1["abc::OArchive<br/>write_archive()"]
        W2["ogawa::OArchive<br/>(implementation)"]
        W3["geom::OPolyMesh<br/>(re-exported)"]
    end

    R1 --> R2 --> R3 --> R4

    W1 -->|"delegates"| W2
    W2 --> W3

    style W1 fill:#9f9
    style R1 fill:#9f9
```

**Status**: ✅ FIXED - abc::OArchive now has write_archive() that delegates to ogawa.

## 14. Remaining Dead Code (Intentional)

```mermaid
pie title Dead Code Analysis (31 #[allow(dead_code)])
    "GPU Resources (held alive)" : 12
    "Future Features (planned)" : 8
    "API Wrappers (intentional)" : 5
    "Unused Fields (refactor)" : 4
    "Reference Code (kept)" : 2
```

## 15. Bug Hunt Findings (2026-01-20)

### Material Inheritance Bug Flow

```mermaid
sequenceDiagram
    participant MC as MeshConverter
    participant Mat as Material Props
    participant Inh as Inheritance Resolver

    Note over MC: CURRENT (BUGGY)
    MC->>Mat: Apply material properties
    Mat-->>MC: Properties applied
    MC->>Inh: resolve_material_inheritance()
    Inh-->>MC: Inheritance resolved (TOO LATE!)

    Note over MC: CORRECT ORDER
    MC->>Inh: resolve_material_inheritance()
    Inh-->>MC: Inheritance resolved FIRST
    MC->>Mat: Apply material properties
    Mat-->>MC: Properties correctly applied
```

**Location:** `src/viewer/mesh_converter.rs:588-598`

### Viewer Scene State Bug

```mermaid
stateDiagram-v2
    [*] --> Empty: Initial
    Empty --> HasCameras: Load file with cameras
    HasCameras --> HasCameras: Load file with cameras (UPDATE)
    HasCameras --> HasCameras: Load file WITHOUT cameras (BUG: stale data!)

    note right of HasCameras
        BUG: Old cameras never cleared
        when loading file without cameras
    end note
```

**Location:** `src/viewer/app.rs:1466-1477`

### Python Object Traversal Performance

```mermaid
flowchart TD
    A[Python: obj.getProperty] --> B[Rust: with_object()]
    B --> C[Get archive root]
    C --> D[Split path into parts]
    D --> E[Loop: for each part]
    E --> F[Find child by name]
    F --> G[Navigate to child]
    G --> E
    E --> H[Finally access property]

    style B fill:#f99
    style E fill:#f99

    Note1[Every method call<br/>repeats this entire flow!]
    Note1 -.-> B
```

**Location:** `src/python/object.rs:38-63`

### Dead Code Distribution

```mermaid
pie showData title Dead Code by Module (31 instances)
    "viewer/renderer" : 15
    "viewer/mesh_converter" : 8
    "viewer/app" : 3
    "ogawa/abc_impl" : 2
    "viewer/environment" : 2
    "viewer/viewport" : 1
```

## 16. Bug Priority Matrix

```mermaid
quadrantChart
    title Bug Severity vs Fix Effort
    x-axis Easy Fix --> Hard Fix
    y-axis Low Impact --> High Impact
    quadrant-1 Do First
    quadrant-2 Plan Carefully
    quadrant-3 Quick Wins
    quadrant-4 Backlog

    Material Inheritance: [0.3, 0.9]
    Exit Bypass: [0.2, 0.85]
    Matrix Convention: [0.4, 0.8]
    Scene Refresh: [0.35, 0.65]
    Vertex Hash: [0.5, 0.6]
    CPU Usage: [0.25, 0.55]
    Python valid(): [0.15, 0.5]
    String Arrays: [0.6, 0.45]
    Missing Constructors: [0.3, 0.4]
    Dead Code: [0.4, 0.2]
    Code Duplication: [0.7, 0.3]
```

## Legend

| Symbol | Meaning |
|--------|---------|
| Solid Arrow | Direct dependency |
| Dashed Arrow | Optional dependency |
| Box | Module/Component |
| Diamond | Decision point |
| Cylinder | Data storage |
| Red fill | Problem/Issue |
| Green fill | Working/Correct |
| Yellow fill | Warning/Stub |
