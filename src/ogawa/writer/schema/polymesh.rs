//! PolyMesh schema writer.
//!
//! References:
//! - `_ref/alembic/lib/Alembic/AbcGeom/OPolyMesh.cpp`
//! - `_ref/alembic/lib/Alembic/AbcGeom/OPolyMesh.h`

use crate::core::MetaData;
use crate::util::{BBox3d, DataType, PlainOldDataType};

use super::super::object::OObject;
use super::super::property::{OProperty, OPropertyData};
use super::util::compute_bounds_vec3;

/// PolyMesh sample data.
pub struct OPolyMeshSample {
    pub positions: Vec<glam::Vec3>,
    pub face_counts: Vec<i32>,
    pub face_indices: Vec<i32>,
    pub velocities: Option<Vec<glam::Vec3>>,
    pub normals: Option<Vec<glam::Vec3>>,
    /// Write normals as simple array (true) or GeomParam compound (false).
    pub normals_is_simple_array: bool,
    pub uvs: Option<Vec<glam::Vec2>>,
    /// Explicit self bounds (if None, computed from positions).
    pub self_bounds: Option<BBox3d>,
}

impl OPolyMeshSample {
    /// Create new sample with required data.
    pub fn new(positions: Vec<glam::Vec3>, face_counts: Vec<i32>, face_indices: Vec<i32>) -> Self {
        Self {
            positions,
            face_counts,
            face_indices,
            velocities: None,
            normals: None,
            normals_is_simple_array: false, // Default to compound format.
            uvs: None,
            self_bounds: None,
        }
    }
}

/// PolyMesh schema writer.
pub struct OPolyMesh {
    object: OObject,
    geom_compound: OProperty,
    arb_geom_compound: Option<OProperty>,
    time_sampling_index: u32,
}

impl OPolyMesh {
    /// Create a new PolyMesh.
    pub fn new(name: &str) -> Self {
        let mut object = OObject::new(name);
        let mut meta = MetaData::new();
        meta.set("schema", "AbcGeom_PolyMesh_v1");
        meta.set("schemaBaseType", "AbcGeom_GeomBase_v1");
        meta.set("schemaObjTitle", "AbcGeom_PolyMesh_v1:.geom");
        object.meta_data = meta;

        let mut geom_meta = MetaData::new();
        geom_meta.set("schema", "AbcGeom_PolyMesh_v1");
        geom_meta.set("schemaBaseType", "AbcGeom_GeomBase_v1");
        let mut geom = OProperty::compound(".geom");
        geom.meta_data = geom_meta;

        Self { object, geom_compound: geom, arb_geom_compound: None, time_sampling_index: 0 }
    }

    /// Set time sampling index for animated properties.
    pub fn with_time_sampling(mut self, index: u32) -> Self {
        self.time_sampling_index = index;
        self
    }

    /// Set time sampling index for animated properties.
    pub fn set_time_sampling(&mut self, index: u32) {
        self.time_sampling_index = index;
    }

    /// Add a sample (positions + topology + optional data).
    /// 
        /// PROPERTY CREATION ORDER matches C++ init():
        ///   .selfBnds, P, .faceIndices, .faceCounts, (velocities), (uvs), (normals)
    /// 
    /// DATA WRITE ORDER matches C++ set():
    ///   P, .faceIndices, .faceCounts, (velocities), .selfBnds, (uvs), (normals)
    pub fn add_sample(&mut self, sample: &OPolyMeshSample) {
        // === STEP 1: Create properties in C++ init() order ===
        // This determines the order of properties in the compound.
        // data_write_order determines the order of keyed_data writes.
        
        // C++ set() DATA WRITE ORDER:
        // 0 = P (positions)
        // 1 = .faceIndices
        // 2 = .faceCounts
        // 3 = .velocities (if present)
        // 4 = .selfBnds
        // 5 = N (normals, if simple array)
        
        // .selfBnds (created first via OGeomBase)
        {
            let sb = self.get_or_create_scalar_with_meta(
                ".selfBnds",
                DataType::new(PlainOldDataType::Float64, 6),
                MetaData::new(),
            );
            sb.data_write_order = 4; // Written AFTER faceIndices/faceCounts
        }
        
        // P (created after .selfBnds in createPositionsProperty)
        let mut p_meta = MetaData::new();
        p_meta.set("geoScope", "vtx");
        p_meta.set("interpretation", "point");
        {
            let p = self.get_or_create_array_with_meta(
                "P",
                DataType::new(PlainOldDataType::Float32, 3),
                p_meta.clone(),
            );
            p.data_write_order = 0;
        }
        
        // .faceIndices (created third in init)
        {
            let fi = self.get_or_create_array_with_ts(
                ".faceIndices",
                DataType::new(PlainOldDataType::Int32, 1),
            );
            fi.data_write_order = 1;
        }
        
        // .faceCounts (created fourth in init)
        {
            let fc = self.get_or_create_array_with_ts(
                ".faceCounts",
                DataType::new(PlainOldDataType::Int32, 1),
            );
            fc.data_write_order = 2;
        }
        
        // Optional properties created in set() before data writes
        if sample.velocities.is_some() {
            let v = self.get_or_create_array_with_ts(
                ".velocities",
                DataType::new(PlainOldDataType::Float32, 3),
            );
            v.data_write_order = 3;
        }
        
        // Normals property created in set() if present
        if sample.normals.is_some()
            && sample.normals_is_simple_array {
                let mut n_meta = MetaData::new();
                n_meta.set("arrayExtent", "1");
                n_meta.set("geoScope", "fvr");
                n_meta.set("interpretation", "normal");
                n_meta.set("isGeomParam", "true");
                n_meta.set("podExtent", "3");
                n_meta.set("podName", "float32_t");
                let n = self.get_or_create_array_with_meta(
                    "N",
                    DataType::new(PlainOldDataType::Float32, 3),
                    n_meta,
                );
                n.data_write_order = 5;
            }
        
        // === STEP 2: Write data ===
        // The actual write order is determined by data_write_order field.
        
        // P data
        let positions_prop = self.get_or_create_array_with_meta(
            "P",
            DataType::new(PlainOldDataType::Float32, 3),
            p_meta,
        );
        positions_prop.add_array_pod(&sample.positions);
        
        // .faceIndices data
        let face_indices_prop = self.get_or_create_array_with_ts(
            ".faceIndices",
            DataType::new(PlainOldDataType::Int32, 1),
        );
        face_indices_prop.add_array_pod(&sample.face_indices);
        
        // .faceCounts data
        let face_counts_prop = self.get_or_create_array_with_ts(
            ".faceCounts",
            DataType::new(PlainOldDataType::Int32, 1),
        );
        face_counts_prop.add_array_pod(&sample.face_counts);
        
        // .selfBnds data
        let bounds = if let Some(ref bnds) = sample.self_bounds {
            [bnds.min.x, bnds.min.y, bnds.min.z, bnds.max.x, bnds.max.y, bnds.max.z]
        } else {
            compute_bounds_vec3(&sample.positions)
        };
        let self_bnds_prop = self.get_or_create_scalar_with_meta(
            ".selfBnds",
            DataType::new(PlainOldDataType::Float64, 6),
            MetaData::new(),
        );
        self_bnds_prop.add_scalar_pod(&bounds);

        // Velocities (optional)
        if let Some(ref vels) = sample.velocities {
            let vel_prop = self.get_or_create_array_with_ts(
                ".velocities",
                DataType::new(PlainOldDataType::Float32, 3),
            );
            vel_prop.add_array_pod(vels);
        }

        // Normals (optional)
        if let Some(ref normals) = sample.normals {
            if sample.normals_is_simple_array {
                let mut n_meta = MetaData::new();
                n_meta.set("arrayExtent", "1");
                n_meta.set("geoScope", "fvr");
                n_meta.set("interpretation", "normal");
                n_meta.set("isGeomParam", "true");
                n_meta.set("podExtent", "3");
                n_meta.set("podName", "float32_t");
                let n_prop = self.get_or_create_array_with_meta(
                    "N",
                    DataType::new(PlainOldDataType::Float32, 3),
                    n_meta,
                );
                n_prop.add_array_pod(normals);
            } else {
                let n_compound = self.geom_compound.get_or_create_compound_child("N");
                n_compound.meta_data.set("isGeomParam", "true");
                n_compound.meta_data.set("podName", "float32_t");
                n_compound.meta_data.set("podExtent", "3");
                n_compound.meta_data.set("geoScope", "fvr");
                let vals_prop = n_compound.get_or_create_array_child(
                    ".vals",
                    DataType::new(PlainOldDataType::Float32, 3),
                );
                vals_prop.time_sampling_index = self.time_sampling_index;
                vals_prop.add_array_pod(normals);
            }
        }

        // UVs (optional)
        if let Some(ref uvs) = sample.uvs {
            if self.arb_geom_compound.is_none() {
                self.arb_geom_compound = Some(OProperty::compound(".arbGeomParams"));
            }
            let arb = self.arb_geom_compound.as_mut().unwrap();
            let uv_compound = arb.get_or_create_compound_child("uv");
            uv_compound.meta_data.set("isGeomParam", "true");
            uv_compound.meta_data.set("podName", "float32_t");
            uv_compound.meta_data.set("podExtent", "2");
            uv_compound.meta_data.set("geoScope", "fvr");
            let vals_prop = uv_compound.get_or_create_array_child(
                ".vals",
                DataType::new(PlainOldDataType::Float32, 2),
            );
            vals_prop.time_sampling_index = self.time_sampling_index;
            vals_prop.add_array_pod(uvs);
        }
    }

    fn get_or_create_array_with_ts(&mut self, prop_name: &str, data_type: DataType) -> &mut OProperty {
        let ts_idx = self.time_sampling_index;
        if let OPropertyData::Compound(children) = &mut self.geom_compound.data {
            if let Some(idx) = children.iter().position(|p| p.name == prop_name) {
                return &mut children[idx];
            }
            let mut prop = OProperty::array(prop_name, data_type);
            prop.time_sampling_index = ts_idx;
            children.push(prop);
            children.last_mut().unwrap()
        } else {
            unreachable!()
        }
    }

    fn get_or_create_array_with_meta(
        &mut self,
        prop_name: &str,
        data_type: DataType,
        meta: MetaData,
    ) -> &mut OProperty {
        let ts_idx = self.time_sampling_index;
        if let OPropertyData::Compound(children) = &mut self.geom_compound.data {
            if let Some(idx) = children.iter().position(|p| p.name == prop_name) {
                return &mut children[idx];
            }
            let mut prop = OProperty::array(prop_name, data_type);
            prop.time_sampling_index = ts_idx;
            prop.meta_data = meta;
            children.push(prop);
            children.last_mut().unwrap()
        } else {
            unreachable!()
        }
    }

    fn get_or_create_scalar_with_meta(
        &mut self,
        prop_name: &str,
        data_type: DataType,
        meta: MetaData,
    ) -> &mut OProperty {
        let ts_idx = self.time_sampling_index;
        if let OPropertyData::Compound(children) = &mut self.geom_compound.data {
            if let Some(idx) = children.iter().position(|p| p.name == prop_name) {
                return &mut children[idx];
            }
            let mut prop = OProperty::scalar(prop_name, data_type);
            prop.time_sampling_index = ts_idx;
            prop.meta_data = meta;
            children.push(prop);
            children.last_mut().unwrap()
        } else {
            unreachable!()
        }
    }

    /// Build the object.
    pub fn build(mut self) -> OObject {
        self.object.properties.push(self.geom_compound);
        if let Some(arb) = self.arb_geom_compound {
            self.object.properties.push(arb);
        }
        self.object
    }

    /// Add child object.
    pub fn add_child(&mut self, child: OObject) {
        self.object.children.push(child);
    }
}
