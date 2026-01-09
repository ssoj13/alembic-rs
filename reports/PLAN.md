# Alembic-RS: Plan for Rust Port of Alembic

## Executive Summary

Alembic is an open-source interchange framework for computer graphics, developed by Sony Pictures Imageworks and Industrial Light & Magic. This document outlines a comprehensive plan for porting Alembic to Rust.

**Goal**: Create a native Rust implementation of Alembic that is:
- Fully compatible with existing .abc files (Ogawa format)
- Memory-safe and thread-safe by design
- Idiomatic Rust with zero-cost abstractions
- Performant (matching or exceeding C++ implementation)

---

## 1. Architecture Overview

### 1.1 Original C++ Layer Structure

```
+------------------------------------------------------------------+
|                        AbcGeom                                    |
|  (PolyMesh, SubD, Curves, Points, NuPatch, Camera, Light, Xform) |
+------------------------------------------------------------------+
|                     AbcMaterial / AbcCollection                   |
+------------------------------------------------------------------+
|                           Abc                                     |
|        (IArchive/OArchive, IObject/OObject, Properties)          |
+------------------------------------------------------------------+
|                      AbcCoreFactory                               |
+------------------------------------------------------------------+
|        AbcCoreOgawa        |        AbcCoreHDF5 (optional)       |
+------------------------------------------------------------------+
|                       AbcCoreAbstract                             |
|            (Interfaces: Archive, Object, Property)                |
+------------------------------------------------------------------+
|            Ogawa               |           HDF5                   |
|    (Low-level binary format)   |    (External dependency)        |
+------------------------------------------------------------------+
|                           Util                                    |
|              (POD types, exceptions, threading)                   |
+------------------------------------------------------------------+
```

### 1.2 Proposed Rust Module Structure

```
alembic-rs/
+-- src/
|   +-- lib.rs                    # Re-exports, prelude
|   +-- util/                     # Basic types and utilities
|   |   +-- mod.rs
|   |   +-- pod.rs                # PlainOldDataType enum + traits
|   |   +-- data_type.rs          # DataType struct
|   |   +-- error.rs              # Error types (thiserror)
|   |   +-- math.rs               # Re-exports from glam
|   +-- ogawa/                    # Low-level Ogawa format
|   |   +-- mod.rs
|   |   +-- reader.rs             # IStreams, IArchive, IGroup, IData
|   |   +-- writer.rs             # OStream, OArchive, OGroup, OData
|   |   +-- format.rs             # Binary format constants/magic
|   +-- core/                     # AbcCoreAbstract + AbcCoreOgawa
|   |   +-- mod.rs
|   |   +-- traits.rs             # Abstract traits (ArchiveReader, etc.)
|   |   +-- time_sampling.rs      # TimeSampling, TimeSamplingType
|   |   +-- metadata.rs           # MetaData
|   |   +-- property_header.rs    # PropertyHeader
|   |   +-- object_header.rs      # ObjectHeader
|   |   +-- array_sample.rs       # ArraySample, ArraySampleKey
|   |   +-- ogawa_reader.rs       # Ogawa implementation of reader traits
|   |   +-- ogawa_writer.rs       # Ogawa implementation of writer traits
|   |   +-- factory.rs            # Auto-detection factory
|   +-- abc/                      # High-level Abc API
|   |   +-- mod.rs
|   |   +-- archive.rs            # IArchive, OArchive
|   |   +-- object.rs             # IObject, OObject
|   |   +-- schema.rs             # Schema traits and macros
|   |   +-- property/
|   |   |   +-- mod.rs
|   |   |   +-- scalar.rs         # IScalarProperty, OScalarProperty
|   |   |   +-- array.rs          # IArrayProperty, OArrayProperty
|   |   |   +-- compound.rs       # ICompoundProperty, OCompoundProperty
|   |   |   +-- typed.rs          # Typed property wrappers
|   |   +-- sample_selector.rs    # ISampleSelector
|   +-- geom/                     # AbcGeom schemas
|   |   +-- mod.rs
|   |   +-- geom_base.rs          # IGeomBase, OGeomBase
|   |   +-- geom_param.rs         # IGeomParam, OGeomParam
|   |   +-- polymesh.rs           # IPolyMesh, OPolyMesh
|   |   +-- subd.rs               # ISubD, OSubD
|   |   +-- curves.rs             # ICurves, OCurves
|   |   +-- points.rs             # IPoints, OPoints
|   |   +-- nupatch.rs            # INuPatch, ONuPatch
|   |   +-- xform.rs              # IXform, OXform, XformOp, XformSample
|   |   +-- camera.rs             # ICamera, OCamera, CameraSample
|   |   +-- light.rs              # ILight, OLight
|   |   +-- faceset.rs            # IFaceSet, OFaceSet
|   |   +-- visibility.rs         # Visibility utilities
|   +-- material/                 # AbcMaterial (optional, Phase 3)
|   |   +-- mod.rs
|   +-- collection/               # AbcCollection (optional, Phase 3)
|   |   +-- mod.rs
+-- examples/
|   +-- abc_echo.rs               # Print archive info
|   +-- abc_ls.rs                 # List archive contents
|   +-- read_mesh.rs              # Read PolyMesh example
|   +-- write_mesh.rs             # Write PolyMesh example
+-- tests/
|   +-- integration/
|   +-- compatibility/            # Test with real .abc files
+-- benches/
+-- Cargo.toml
```

---

## 2. Dependencies Analysis

### 2.1 Required Crates

| Crate | Purpose | Notes |
|-------|---------|-------|
| `half` | f16 (float16) support | IEEE 754 half-precision |
| `memmap2` | Memory-mapped file I/O | For efficient large file reading |
| `glam` | 3D math (Vec3, Mat4, Quat) | Fast, game-oriented |
| `byteorder` | Endian-aware binary I/O | Little/Big endian |
| `thiserror` | Error handling | Derive Error trait |
| `parking_lot` | Fast mutexes/rwlocks | Better than std |
| `smallvec` | Small vector optimization | For metadata strings |

### 2.2 Optional Crates

| Crate | Purpose | Notes |
|-------|---------|-------|
| `rayon` | Parallel iteration | For multi-threaded reading |
| `serde` | Serialization | For metadata/config |
| `hdf5` | HDF5 backend | Legacy support (consider) |

### 2.3 Proposed Cargo.toml

```toml
[package]
name = "alembic"
version = "0.1.0"
edition = "2021"
license = "BSD-3-Clause"
description = "Rust implementation of Alembic (.abc) 3D interchange format"
repository = "https://github.com/..."
keywords = ["3d", "graphics", "animation", "vfx", "alembic"]
categories = ["multimedia", "graphics", "parser-implementations"]

[dependencies]
half = "2.4"
memmap2 = "0.9"
glam = { version = "0.28", features = ["bytemuck"] }
byteorder = "1.5"
thiserror = "2.0"
parking_lot = "0.12"
smallvec = "1.13"

[dev-dependencies]
criterion = "0.5"
tempfile = "3"

[features]
default = ["mmap"]
mmap = []  # Memory-mapped file support
parallel = ["rayon"]  # Parallel reading
serde = ["dep:serde", "glam/serde"]

[[bench]]
name = "read_benchmark"
harness = false
```

---

## 3. Type System Mapping

### 3.1 PlainOldDataType (POD)

| C++ Type | Rust Type | POD Enum |
|----------|-----------|----------|
| `bool_t` | `bool` (stored as u8) | `Boolean` |
| `uint8_t` | `u8` | `Uint8` |
| `int8_t` | `i8` | `Int8` |
| `uint16_t` | `u16` | `Uint16` |
| `int16_t` | `i16` | `Int16` |
| `uint32_t` | `u32` | `Uint32` |
| `int32_t` | `i32` | `Int32` |
| `uint64_t` | `u64` | `Uint64` |
| `int64_t` | `i64` | `Int64` |
| `float16_t` | `half::f16` | `Float16` |
| `float32_t` | `f32` | `Float32` |
| `float64_t` | `f64` | `Float64` |
| `std::string` | `String` | `String` |
| `std::wstring` | `String` (UTF-8) | `Wstring` |

### 3.2 Math Types (using glam)

| Alembic Type | Rust Type |
|--------------|-----------|
| `V2f` | `glam::Vec2` |
| `V3f` | `glam::Vec3` |
| `V4f` | `glam::Vec4` |
| `V2d` | `glam::DVec2` |
| `V3d` | `glam::DVec3` |
| `V4d` | `glam::DVec4` |
| `M33f` | `glam::Mat3` |
| `M44f` | `glam::Mat4` |
| `M33d` | `glam::DMat3` |
| `M44d` | `glam::DMat4` |
| `Quatf` | `glam::Quat` |
| `Quatd` | `glam::DQuat` |
| `Box3f` | `(Vec3, Vec3)` or custom `BBox3` |
| `Box3d` | `(DVec3, DVec3)` or custom `BBox3d` |

### 3.3 DataType Definition

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DataType {
    pub pod: PlainOldDataType,
    pub extent: u8,  // 1 for scalar, N for VecN
}

impl DataType {
    pub const fn new(pod: PlainOldDataType, extent: u8) -> Self {
        Self { pod, extent }
    }

    pub fn num_bytes(&self) -> usize {
        self.pod.num_bytes() * self.extent as usize
    }
}

// Predefined types
pub const BOOL: DataType = DataType::new(PlainOldDataType::Boolean, 1);
pub const FLOAT32: DataType = DataType::new(PlainOldDataType::Float32, 1);
pub const VEC3F: DataType = DataType::new(PlainOldDataType::Float32, 3);
pub const MAT44F: DataType = DataType::new(PlainOldDataType::Float32, 16);
// ... etc
```

---

## 4. Ogawa Binary Format

### 4.1 File Structure

```
+------------------+
| Magic: "Ogawa"   |  5 bytes
+------------------+
| Frozen flag      |  1 byte (0x00 or 0xFF)
+------------------+
| Version          |  2 bytes (u16 LE)
+------------------+
| Root Group Pos   |  8 bytes (u64 LE)
+------------------+
| ... Data ...     |
+------------------+
```

### 4.2 Group Structure

Groups contain children which can be either:
- **Data** - raw bytes (leaf nodes)
- **Group** - nested groups (branch nodes)

```
+------------------+
| Num Children     |  8 bytes (u64 LE)
+------------------+
| Child 0 offset   |  8 bytes (u64 LE, MSB = isGroup flag)
+------------------+
| Child 1 offset   |  ...
+------------------+
| ... more children
+------------------+
```

### 4.3 Data Structure

```
+------------------+
| Size             |  8 bytes (u64 LE)
+------------------+
| Raw bytes        |  [Size] bytes
+------------------+
```

### 4.4 Rust Reader Implementation

```rust
pub struct OgawaReader {
    mmap: Mmap,  // or File for non-mmap mode
    version: u16,
    frozen: bool,
    root_pos: u64,
}

impl OgawaReader {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, Error> {
        let file = File::open(path)?;
        let mmap = unsafe { Mmap::map(&file)? };

        // Validate magic
        if &mmap[0..5] != b"Ogawa" {
            return Err(Error::InvalidMagic);
        }

        let frozen = mmap[5] == 0xFF;
        let version = u16::from_le_bytes([mmap[6], mmap[7]]);
        let root_pos = u64::from_le_bytes(mmap[8..16].try_into()?);

        Ok(Self { mmap, version, frozen, root_pos })
    }

    pub fn root_group(&self) -> GroupReader<'_> {
        GroupReader::new(&self.mmap, self.root_pos)
    }
}
```

---

## 5. Core Abstract Layer

### 5.1 Trait Definitions

```rust
// Archive traits
pub trait ArchiveReader {
    fn name(&self) -> &str;
    fn num_time_samplings(&self) -> usize;
    fn time_sampling(&self, index: usize) -> Option<&TimeSampling>;
    fn root(&self) -> &dyn ObjectReader;
}

pub trait ArchiveWriter {
    fn name(&self) -> &str;
    fn add_time_sampling(&mut self, ts: TimeSampling) -> u32;
    fn root(&mut self) -> &mut dyn ObjectWriter;
}

// Object traits
pub trait ObjectReader {
    fn header(&self) -> &ObjectHeader;
    fn parent(&self) -> Option<&dyn ObjectReader>;
    fn num_children(&self) -> usize;
    fn child(&self, index: usize) -> Option<Box<dyn ObjectReader>>;
    fn child_by_name(&self, name: &str) -> Option<Box<dyn ObjectReader>>;
    fn properties(&self) -> &dyn CompoundPropertyReader;
}

// Property traits
pub trait PropertyReader {
    fn header(&self) -> &PropertyHeader;
    fn is_scalar(&self) -> bool;
    fn is_array(&self) -> bool;
    fn is_compound(&self) -> bool;
}

pub trait ScalarPropertyReader: PropertyReader {
    fn num_samples(&self) -> usize;
    fn read_sample(&self, index: usize, out: &mut [u8]) -> Result<(), Error>;
}

pub trait ArrayPropertyReader: PropertyReader {
    fn num_samples(&self) -> usize;
    fn sample_size(&self, index: usize) -> usize;
    fn read_sample(&self, index: usize) -> Result<Vec<u8>, Error>;
}

pub trait CompoundPropertyReader: PropertyReader {
    fn num_properties(&self) -> usize;
    fn property(&self, index: usize) -> Option<Box<dyn PropertyReader>>;
    fn property_by_name(&self, name: &str) -> Option<Box<dyn PropertyReader>>;
}
```

### 5.2 TimeSampling

```rust
#[derive(Clone, Debug)]
pub enum TimeSamplingType {
    /// Single static sample at time 0
    Identity,
    /// Uniform sampling: start_time + index * time_per_cycle
    Uniform { time_per_cycle: f64, start_time: f64 },
    /// Cyclic: repeating pattern of times
    Cyclic { time_per_cycle: f64, times: Vec<f64> },
    /// Acyclic: explicit time for each sample
    Acyclic { times: Vec<f64> },
}

#[derive(Clone, Debug)]
pub struct TimeSampling {
    pub sampling_type: TimeSamplingType,
}

impl TimeSampling {
    pub fn sample_time(&self, index: usize, num_samples: usize) -> f64 {
        match &self.sampling_type {
            TimeSamplingType::Identity => 0.0,
            TimeSamplingType::Uniform { time_per_cycle, start_time } => {
                *start_time + (index as f64) * *time_per_cycle
            }
            TimeSamplingType::Cyclic { time_per_cycle, times } => {
                let cycle = index / times.len();
                let local_idx = index % times.len();
                times[local_idx] + (cycle as f64) * *time_per_cycle
            }
            TimeSamplingType::Acyclic { times } => {
                times.get(index).copied().unwrap_or(0.0)
            }
        }
    }

    pub fn floor_index(&self, time: f64, num_samples: usize) -> (usize, f64) {
        // ... binary search implementation
    }
}
```

---

## 6. High-Level Abc API

### 6.1 IArchive / OArchive

```rust
pub struct IArchive {
    reader: Box<dyn ArchiveReader>,
}

impl IArchive {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, Error> {
        let reader = detect_and_open(path)?;  // Factory
        Ok(Self { reader })
    }

    pub fn root(&self) -> IObject<'_> {
        IObject::new(self.reader.root())
    }

    pub fn time_sampling(&self, index: u32) -> Option<&TimeSampling> {
        self.reader.time_sampling(index as usize)
    }
}

pub struct OArchive {
    writer: Box<dyn ArchiveWriter>,
}

impl OArchive {
    pub fn create(path: impl AsRef<Path>) -> Result<Self, Error> {
        let writer = OgawaWriter::create(path)?;
        Ok(Self { writer: Box::new(writer) })
    }

    pub fn root(&mut self) -> OObject<'_> {
        OObject::new(self.writer.root())
    }
}
```

### 6.2 IObject / OObject

```rust
pub struct IObject<'a> {
    reader: &'a dyn ObjectReader,
}

impl<'a> IObject<'a> {
    pub fn name(&self) -> &str {
        &self.reader.header().name
    }

    pub fn full_name(&self) -> String {
        // Build path from root
    }

    pub fn num_children(&self) -> usize {
        self.reader.num_children()
    }

    pub fn child(&self, index: usize) -> Option<IObject<'a>> {
        // ...
    }

    pub fn properties(&self) -> ICompoundProperty<'a> {
        ICompoundProperty::new(self.reader.properties())
    }

    /// Check if this object has a specific schema
    pub fn has_schema<S: Schema>(&self) -> bool {
        // Check metadata for schema match
    }

    /// Get typed schema if available
    pub fn as_schema<S: Schema>(&self) -> Option<S::Reader<'a>> {
        if self.has_schema::<S>() {
            Some(S::Reader::new(self.properties()))
        } else {
            None
        }
    }
}
```

### 6.3 Properties

```rust
pub struct IScalarProperty<'a, T> {
    reader: &'a dyn ScalarPropertyReader,
    _phantom: PhantomData<T>,
}

impl<'a, T: Pod> IScalarProperty<'a, T> {
    pub fn num_samples(&self) -> usize {
        self.reader.num_samples()
    }

    pub fn get(&self, selector: impl Into<SampleSelector>) -> Result<T, Error> {
        let index = selector.into().resolve(self.num_samples());
        let mut buf = [0u8; std::mem::size_of::<T>()];
        self.reader.read_sample(index, &mut buf)?;
        Ok(T::from_le_bytes(&buf))
    }
}

pub struct IArrayProperty<'a, T> {
    reader: &'a dyn ArrayPropertyReader,
    _phantom: PhantomData<T>,
}

impl<'a, T: Pod + Clone> IArrayProperty<'a, T> {
    pub fn num_samples(&self) -> usize {
        self.reader.num_samples()
    }

    pub fn get(&self, selector: impl Into<SampleSelector>) -> Result<Vec<T>, Error> {
        let index = selector.into().resolve(self.num_samples());
        let bytes = self.reader.read_sample(index)?;
        // Convert bytes to Vec<T>
        Ok(bytemuck::cast_slice(&bytes).to_vec())
    }
}
```

---

## 7. AbcGeom Schemas

### 7.1 Schema Trait

```rust
pub trait Schema {
    const TITLE: &'static str;
    const BASE_TITLE: &'static str;
    const DEFAULT_NAME: &'static str;

    type Reader<'a>: SchemaReader<'a>;
    type Writer<'a>: SchemaWriter<'a>;
    type Sample: Default;
}

pub trait SchemaReader<'a> {
    type Sample;

    fn get(&self, selector: impl Into<SampleSelector>) -> Result<Self::Sample, Error>;
    fn num_samples(&self) -> usize;
    fn time_sampling(&self) -> &TimeSampling;
}
```

### 7.2 PolyMesh Schema

```rust
pub struct PolyMeshSchemaInfo;

impl Schema for PolyMeshSchemaInfo {
    const TITLE: &'static str = "AbcGeom_PolyMesh_v1";
    const BASE_TITLE: &'static str = "AbcGeom_GeomBase_v1";
    const DEFAULT_NAME: &'static str = ".geom";

    type Reader<'a> = IPolyMeshSchema<'a>;
    type Writer<'a> = OPolyMeshSchema<'a>;
    type Sample = PolyMeshSample;
}

#[derive(Default, Clone)]
pub struct PolyMeshSample {
    pub positions: Vec<Vec3>,
    pub face_indices: Vec<i32>,
    pub face_counts: Vec<i32>,
    pub velocities: Option<Vec<Vec3>>,
    pub self_bounds: BBox3d,
}

pub struct IPolyMeshSchema<'a> {
    props: ICompoundProperty<'a>,
    positions: IArrayProperty<'a, Vec3>,
    indices: IArrayProperty<'a, i32>,
    counts: IArrayProperty<'a, i32>,
    velocities: Option<IArrayProperty<'a, Vec3>>,
    uvs: Option<IGeomParam<'a, Vec2>>,
    normals: Option<IGeomParam<'a, Vec3>>,
}

impl<'a> IPolyMeshSchema<'a> {
    pub fn get(&self, selector: impl Into<SampleSelector>) -> Result<PolyMeshSample, Error> {
        let sel = selector.into();
        Ok(PolyMeshSample {
            positions: self.positions.get(sel)?,
            face_indices: self.indices.get(sel)?,
            face_counts: self.counts.get(sel)?,
            velocities: self.velocities.as_ref().map(|v| v.get(sel)).transpose()?,
            self_bounds: self.self_bounds.get(sel)?,
        })
    }

    pub fn uvs(&self) -> Option<&IGeomParam<'a, Vec2>> {
        self.uvs.as_ref()
    }

    pub fn normals(&self) -> Option<&IGeomParam<'a, Vec3>> {
        self.normals.as_ref()
    }
}
```

### 7.3 Xform Schema

```rust
#[derive(Clone, Debug)]
pub enum XformOp {
    Scale(Vec3),
    Translate(Vec3),
    RotateX(f64),
    RotateY(f64),
    RotateZ(f64),
    RotateXYZ { xyz: Vec3, order: RotationOrder },
    Matrix(Mat4),
}

#[derive(Default, Clone)]
pub struct XformSample {
    pub ops: Vec<XformOp>,
    pub inherits: bool,
}

impl XformSample {
    pub fn matrix(&self) -> Mat4 {
        let mut m = Mat4::IDENTITY;
        for op in &self.ops {
            m = m * op.to_matrix();
        }
        m
    }
}

pub struct IXformSchema<'a> {
    // ...
}
```

### 7.4 Other Schemas (Structure)

```rust
// Curves
pub struct CurvesSample {
    pub positions: Vec<Vec3>,
    pub num_vertices: Vec<i32>,
    pub curve_type: CurveType,
    pub wrap: CurvePeriodicity,
    pub basis: BasisType,
    pub knots: Option<Vec<f32>>,
    pub orders: Option<Vec<u8>>,
    pub widths: Option<GeomParamSample<f32>>,
    pub uvs: Option<GeomParamSample<Vec2>>,
    pub normals: Option<GeomParamSample<Vec3>>,
}

// Points
pub struct PointsSample {
    pub positions: Vec<Vec3>,
    pub ids: Vec<u64>,
    pub velocities: Option<Vec<Vec3>>,
    pub widths: Option<GeomParamSample<f32>>,
}

// SubD
pub struct SubDSample {
    pub positions: Vec<Vec3>,
    pub face_indices: Vec<i32>,
    pub face_counts: Vec<i32>,
    pub crease_indices: Option<Vec<i32>>,
    pub crease_lengths: Option<Vec<i32>>,
    pub crease_sharpnesses: Option<Vec<f32>>,
    pub corner_indices: Option<Vec<i32>>,
    pub corner_sharpnesses: Option<Vec<f32>>,
    pub subdivision_scheme: SubdivisionScheme,
}

// Camera
pub struct CameraSample {
    pub focal_length: f64,
    pub horizontal_aperture: f64,
    pub vertical_aperture: f64,
    pub horizontal_film_offset: f64,
    pub vertical_film_offset: f64,
    pub lens_squeeze_ratio: f64,
    pub near_clipping_plane: f64,
    pub far_clipping_plane: f64,
    // ... more fields
}
```

---

## 8. Implementation Phases

> **NOTE**: User confirmed: Read + Write in parallel, Full schema set required.

### Phase 1: Foundation (Weeks 1-2)

1. **util module**
   - [ ] PlainOldDataType enum with all variants
   - [ ] DataType struct
   - [ ] Error types with thiserror
   - [ ] Math type re-exports (glam) - **CONFIRMED**

2. **ogawa module (read + write)**
   - [ ] Magic/header parsing & writing
   - [ ] Memory-mapped file support (reading)
   - [ ] IStreams / OStream
   - [ ] IGroup / OGroup (recursive)
   - [ ] IData / OData
   - [ ] IArchive / OArchive

3. **Basic tests**
   - [ ] Unit tests for POD types
   - [ ] Parse simple .abc files
   - [ ] Write simple .abc files
   - [ ] Round-trip test

### Phase 2: Core Layer (Weeks 3-5)

1. **core module - Common**
   - [ ] TimeSampling and TimeSamplingType
   - [ ] MetaData parsing/serialization
   - [ ] PropertyHeader/ObjectHeader
   - [ ] Abstract traits (Reader + Writer)

2. **core/ogawa_reader**
   - [ ] ArchiveReader implementation
   - [ ] ObjectReader implementation
   - [ ] ScalarPropertyReader
   - [ ] ArrayPropertyReader
   - [ ] CompoundPropertyReader

3. **core/ogawa_writer**
   - [ ] ArchiveWriter implementation
   - [ ] ObjectWriter
   - [ ] ScalarPropertyWriter
   - [ ] ArrayPropertyWriter
   - [ ] CompoundPropertyWriter

### Phase 3: Abc API (Weeks 6-7)

1. **abc module (reading)**
   - [ ] IArchive
   - [ ] IObject with child iteration
   - [ ] IScalarProperty<T>
   - [ ] IArrayProperty<T>
   - [ ] ICompoundProperty
   - [ ] SampleSelector

2. **abc module (writing)**
   - [ ] OArchive
   - [ ] OObject
   - [ ] OScalarProperty<T>
   - [ ] OArrayProperty<T>
   - [ ] OCompoundProperty

### Phase 4: AbcGeom - Full Schema Set (Weeks 8-12)

> **CONFIRMED**: Implement all geometry schemas

1. **geom module - common**
   - [ ] GeomScope enum
   - [ ] IGeomBase / OGeomBase
   - [ ] IGeomParam<T> / OGeomParam<T>
   - [ ] Visibility utilities

2. **PolyMesh** (high priority)
   - [ ] IPolyMesh + PolyMeshSample
   - [ ] OPolyMesh
   - [ ] UVs, Normals support

3. **Xform** (high priority)
   - [ ] IXform + XformSample
   - [ ] OXform
   - [ ] XformOp (all operations)

4. **SubD** (subdivision surfaces)
   - [ ] ISubD + SubDSample
   - [ ] OSubD
   - [ ] Creases, corners, holes

5. **Curves**
   - [ ] ICurves + CurvesSample
   - [ ] OCurves
   - [ ] CurveType, BasisType

6. **Points**
   - [ ] IPoints + PointsSample
   - [ ] OPoints
   - [ ] IDs, velocities, widths

7. **NuPatch** (NURBS surfaces)
   - [ ] INuPatch + NuPatchSample
   - [ ] ONuPatch
   - [ ] Knots, orders, trim curves

8. **Camera**
   - [ ] ICamera + CameraSample
   - [ ] OCamera
   - [ ] All camera parameters

9. **Light**
   - [ ] ILight
   - [ ] OLight
   - [ ] Schema properties

10. **FaceSet**
    - [ ] IFaceSet + FaceSetSample
    - [ ] OFaceSet
    - [ ] Exclusivity

### Phase 5: Examples, Tests, Polish (Weeks 13-14)

1. **Examples**
   - [ ] abc_echo - Print archive info
   - [ ] abc_ls - List archive contents (tree)
   - [ ] read_mesh - Read PolyMesh example
   - [ ] write_mesh - Write PolyMesh example
   - [ ] read_animation - Read animated transform
   - [ ] write_animation - Write animated data

2. **Compatibility Tests**
   - [ ] Tests with Maya-exported files
   - [ ] Tests with Houdini-exported files
   - [ ] Tests with Blender-exported files
   - [ ] Round-trip tests (write -> read -> compare)

3. **Performance**
   - [ ] Benchmarks vs C++ implementation
   - [ ] Memory profiling
   - [ ] Large file handling tests

4. **Documentation**
   - [ ] API documentation (rustdoc)
   - [ ] README with examples
   - [ ] Migration guide from C++ API

### Phase 6: Optional Extensions (Future)

1. **AbcMaterial** (optional)
   - [ ] IMaterial / OMaterial

2. **AbcCollection** (optional)
   - [ ] ICollections / OCollections

3. **AbcCoreLayer** (optional)
   - [ ] Layered archive reading

4. **Performance**
   - [ ] Benchmarks
   - [ ] Parallel reading (rayon)
   - [ ] Memory optimization

5. **Documentation**
   - [ ] API docs
   - [ ] Examples
   - [ ] README

---

## 9. Design Decisions

### 9.1 HDF5 Support

**Decision**: ~~Skip HDF5 backend initially.~~ **CONFIRMED: Ogawa only.**

**Rationale**:
- Ogawa is the modern, recommended format
- HDF5 adds significant complexity and external dependency
- Most modern .abc files use Ogawa
- **User confirmed**: No HDF5 support needed

### 9.2 Thread Safety

**Decision**: Use `parking_lot` for mutexes, design for concurrent read access.

**Approach**:
- Archives are `Send + Sync`
- Reading is lock-free where possible
- Writing requires exclusive access
- Consider `Arc<RwLock<...>>` for shared mutable state

### 9.3 Memory Management

**Decision**: Use memory-mapped files by default with fallback to buffered I/O.

**Rationale**:
- Large .abc files can be gigabytes
- mmap provides efficient random access
- OS handles caching
- Fallback needed for streams/pipes

### 9.4 Error Handling

**Decision**: Use `thiserror` with comprehensive error enum.

```rust
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Invalid Alembic file: {0}")]
    InvalidFile(String),

    #[error("Invalid magic bytes")]
    InvalidMagic,

    #[error("Unsupported version: {0}")]
    UnsupportedVersion(u16),

    #[error("Property not found: {0}")]
    PropertyNotFound(String),

    #[error("Object not found: {0}")]
    ObjectNotFound(String),

    #[error("Type mismatch: expected {expected}, got {actual}")]
    TypeMismatch { expected: String, actual: String },

    #[error("Sample index out of bounds: {index} >= {count}")]
    SampleOutOfBounds { index: usize, count: usize },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
```

### 9.5 API Style

**Decision**: Provide both low-level and high-level APIs.

- **Low-level** (`core` module): Direct trait implementations, maximum flexibility
- **High-level** (`abc`, `geom` modules): Ergonomic, typed wrappers

---

## 10. Compatibility Considerations

### 10.1 File Format Versions

- Support Ogawa format versions 1-N
- Detect version from header
- Graceful degradation for unknown features

### 10.2 Schema Versions

- Parse schema version from metadata
- Support multiple versions where practical
- Log warnings for unknown schemas

### 10.3 Testing Strategy

1. **Unit tests**: Core functionality
2. **Integration tests**: Full read/write cycles
3. **Compatibility tests**:
   - Maya-exported files
   - Houdini-exported files
   - Blender-exported files
   - Original Alembic lib exports
4. **Round-trip tests**: Write -> Read -> Compare

---

## 11. Open Questions

1. **wstring handling**: Convert to UTF-8 String or keep wide strings?
   - *Proposal*: Convert to UTF-8, Alembic rarely uses wstring

2. **Lazy vs eager loading**: Load entire archive or on-demand?
   - *Proposal*: Lazy loading with caching

3. **Zero-copy reading**: Return slices where possible?
   - *Proposal*: Yes, for mmap mode. Use `Cow<[T]>` for flexibility.

4. **Python bindings**: Plan for PyO3 bindings?
   - *Proposal*: Design API to be binding-friendly, add pyo3 later

5. **Async support**: Worth adding async file I/O?
   - *Proposal*: Not initially, can be added with `tokio` later

---

## 12. Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Ogawa format undocumented details | Medium | High | Reverse-engineer from C++ source, test extensively |
| Performance worse than C++ | Low | Medium | Profile early, use unsafe where justified |
| Incomplete schema coverage | Medium | Medium | Prioritize common schemas, add others iteratively |
| Breaking changes in dependencies | Low | Low | Pin versions, audit updates |
| Memory safety in mmap usage | Medium | High | Careful unsafe code review, extensive testing |

---

## 13. Success Criteria

1. **Functional**: Read and write .abc files compatible with Maya, Houdini, Blender
2. **Performance**: Within 20% of C++ implementation for read operations
3. **Safe**: No undefined behavior, all unsafe code audited
4. **Documented**: Complete API documentation with examples
5. **Tested**: >80% code coverage, compatibility test suite

---

## Appendix A: Reference Files

- Original Alembic source: `_ref/alembic/`
- Key headers:
  - `lib/Alembic/Util/PlainOldDataType.h` - POD types
  - `lib/Alembic/AbcCoreAbstract/DataType.h` - DataType
  - `lib/Alembic/AbcCoreAbstract/TimeSampling.h` - Time sampling
  - `lib/Alembic/Ogawa/*.h` - Binary format
  - `lib/Alembic/AbcGeom/*.h` - Geometry schemas

## Appendix B: Useful Resources

- [Alembic official docs](http://docs.alembic.io/)
- [Alembic GitHub](https://github.com/alembic/alembic)
- [half crate docs](https://docs.rs/half/)
- [glam crate docs](https://docs.rs/glam/)
- [memmap2 crate docs](https://docs.rs/memmap2/)
