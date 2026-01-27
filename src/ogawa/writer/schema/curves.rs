//! Curves schema writer.
//!
//! References:
//! - `_ref/alembic/lib/Alembic/AbcGeom/OCurves.cpp`
//! - `_ref/alembic/lib/Alembic/AbcGeom/OCurves.h`

use crate::core::MetaData;
use crate::geom::{BasisType, CurvePeriodicity, CurveType};
use crate::util::{DataType, PlainOldDataType};

use super::super::object::OObject;
use super::super::property::{OProperty, OPropertyData};
use super::util::{bounds_meta, compute_bounds_vec3};

/// Curves sample data for output.
pub struct OCurvesSample {
    pub positions: Vec<glam::Vec3>,
    pub num_vertices: Vec<i32>,
    pub curve_type: CurveType,
    pub wrap: CurvePeriodicity,
    pub basis: BasisType,
    pub velocities: Option<Vec<glam::Vec3>>,
    pub widths: Option<Vec<f32>>,
    pub normals: Option<Vec<glam::Vec3>>,
    pub uvs: Option<Vec<glam::Vec2>>,
    pub knots: Option<Vec<f32>>,
    pub orders: Option<Vec<i32>>,
}

impl OCurvesSample {
    /// Create new curves sample.
    pub fn new(positions: Vec<glam::Vec3>, num_vertices: Vec<i32>) -> Self {
        Self {
            positions,
            num_vertices,
            curve_type: CurveType::Linear,
            wrap: CurvePeriodicity::NonPeriodic,
            basis: BasisType::NoBasis,
            velocities: None,
            widths: None,
            normals: None,
            uvs: None,
            knots: None,
            orders: None,
        }
    }

    /// Set curve type.
    pub fn with_curve_type(mut self, ct: CurveType) -> Self {
        self.curve_type = ct;
        self
    }

    /// Set periodicity.
    pub fn with_wrap(mut self, wrap: CurvePeriodicity) -> Self {
        self.wrap = wrap;
        self
    }

    /// Set basis type.
    pub fn with_basis(mut self, basis: BasisType) -> Self {
        self.basis = basis;
        self
    }
}

/// Curves schema writer.
pub struct OCurves {
    object: OObject,
    geom_compound: OProperty,
    time_sampling_index: u32,
}

impl OCurves {
    /// Create new Curves.
    pub fn new(name: &str) -> Self {
        let mut object = OObject::new(name);
        let mut meta = MetaData::new();
        meta.set("schema", "AbcGeom_Curve_v2");
        meta.set("schemaBaseType", "AbcGeom_GeomBase_v1");
        meta.set("schemaObjTitle", "AbcGeom_Curve_v2:.geom");
        object.meta_data = meta;

        let mut geom_meta = MetaData::new();
        geom_meta.set("schema", "AbcGeom_Curve_v2");
        geom_meta.set("schemaBaseType", "AbcGeom_GeomBase_v1");
        let mut geom = OProperty::compound(".geom");
        geom.meta_data = geom_meta;

        Self { object, geom_compound: geom, time_sampling_index: 0 }
    }

    /// Set time sampling index for animated properties.
    pub fn set_time_sampling(&mut self, index: u32) {
        self.time_sampling_index = index;
    }

    /// Add a sample.
    pub fn add_sample(&mut self, sample: &OCurvesSample) {
        // .selfBnds is created by OGeomBase before P in C++.
        let bounds = compute_bounds_vec3(&sample.positions);
        let self_bnds_prop = self.geom_compound.get_or_create_scalar_child(
            ".selfBnds",
            DataType::new(PlainOldDataType::Float64, 6),
        );
        self_bnds_prop.meta_data = bounds_meta();
        self_bnds_prop.time_sampling_index = self.time_sampling_index;
        self_bnds_prop.data_write_order = 6;
        self_bnds_prop.add_scalar_pod(&bounds);

        let p_prop = self.get_or_create_array_with_meta(
            "P",
            DataType::new(PlainOldDataType::Float32, 3),
            Self::p_meta(),
        );
        p_prop.data_write_order = 0;
        p_prop.add_array_pod(&sample.positions);

        let nverts = self.geom_compound.get_or_create_array_child(
            "nVertices",
            DataType::new(PlainOldDataType::Int32, 1),
        );
        nverts.data_write_order = 1;
        nverts.add_array_pod(&sample.num_vertices);

        // curveBasisAndType: 4 x uint8 scalar [type, wrap, basis, basis]
        // C++ ref: OCurves.cpp:582 calcBasisAndType — byte[3] duplicates byte[2]
        let cbt = self.geom_compound.get_or_create_scalar_child(
            "curveBasisAndType",
            DataType::new(PlainOldDataType::Uint8, 4),
        );
        cbt.data_write_order = 2;
        let basis_u8 = sample.basis.to_u8();
        let basisandtype: [u8; 4] = [
            sample.curve_type.to_u8(),
            sample.wrap.to_u8(),
            basis_u8,
            basis_u8, // duplicated per C++ convention
        ];
        cbt.add_scalar_pod(&basisandtype);

        if let Some(ref vels) = sample.velocities {
            let prop = self.geom_compound.get_or_create_array_child(
                ".velocities",
                DataType::new(PlainOldDataType::Float32, 3),
            );
            prop.data_write_order = 5;
            prop.add_array_pod(vels);
        }

        // C++ ref: OCurves.cpp:511 uses "width" (GeomParam, no dot, singular)
        if let Some(ref widths) = sample.widths {
            let prop = self.geom_compound.get_or_create_array_child(
                "width",
                DataType::new(PlainOldDataType::Float32, 1),
            );
            prop.data_write_order = 9;
            prop.add_array_pod(widths);
        }

        if let Some(ref normals) = sample.normals {
            let prop = self.geom_compound.get_or_create_array_child(
                "N",
                DataType::new(PlainOldDataType::Float32, 3),
            );
            prop.data_write_order = 8;
            prop.add_array_pod(normals);
        }

        if let Some(ref uvs) = sample.uvs {
            let prop = self.geom_compound.get_or_create_array_child(
                "uv",
                DataType::new(PlainOldDataType::Float32, 2),
            );
            prop.data_write_order = 7;
            prop.add_array_pod(uvs);
        }

        if let Some(ref knots) = sample.knots {
            let prop = self.geom_compound.get_or_create_array_child(
                ".knots",
                DataType::new(PlainOldDataType::Float32, 1),
            );
            prop.data_write_order = 11;
            prop.add_array_pod(knots);
        }

        if let Some(ref orders) = sample.orders {
            let prop = self.geom_compound.get_or_create_array_child(
                ".orders",
                DataType::new(PlainOldDataType::Int32, 1),
            );
            prop.data_write_order = 10;
            prop.add_array_pod(orders);
        }
    }

    fn p_meta() -> MetaData {
        let mut meta = MetaData::new();
        meta.set("geoScope", "vtx");
        meta.set("interpretation", "point");
        meta
    }

    fn get_or_create_array_with_meta(&mut self, name: &str, dt: DataType, meta: MetaData) -> &mut OProperty {
        if let OPropertyData::Compound(children) = &mut self.geom_compound.data {
            if let Some(idx) = children.iter().position(|p| p.name == name) {
                return &mut children[idx];
            }
            let mut prop = OProperty::array(name, dt);
            prop.meta_data = meta;
            prop.time_sampling_index = self.time_sampling_index;
            children.push(prop);
            children.last_mut().unwrap()
        } else {
            unreachable!()
        }
    }

    /// Build the object.
    pub fn build(mut self) -> OObject {
        self.object.properties.push(self.geom_compound);
        self.object
    }

    /// Add child object.
    pub fn add_child(&mut self, child: OObject) {
        self.object.children.push(child);
    }
}
