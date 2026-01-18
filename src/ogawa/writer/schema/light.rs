//! Light schema writer.
//!
//! References:
//! - `_ref/alembic/lib/Alembic/AbcGeom/OLight.cpp`
//! - `_ref/alembic/lib/Alembic/AbcGeom/OLight.h`

use crate::core::MetaData;
use crate::geom::CameraSample;
use crate::util::{DataType, PlainOldDataType};

use super::super::object::OObject;
use super::super::property::{OProperty, OPropertyData};

/// Light schema writer.
pub struct OLight {
    object: OObject,
    camera_samples: Vec<CameraSample>,
    time_sampling_index: u32,
}

impl OLight {
    /// Create new Light.
    pub fn new(name: &str) -> Self {
        let mut object = OObject::new(name);
        let mut meta = MetaData::new();
        meta.set("schema", "AbcGeom_Light_v1");
        meta.set("schemaObjTitle", "AbcGeom_Light_v1:.geom");
        object.meta_data = meta;

        Self { object, camera_samples: Vec::new(), time_sampling_index: 0 }
    }

    /// Set time sampling index for animated properties.
    pub fn set_time_sampling(&mut self, index: u32) {
        self.time_sampling_index = index;
    }

    /// Add a camera sample (light parameters stored as camera).
    pub fn add_camera_sample(&mut self, sample: CameraSample) {
        self.camera_samples.push(sample);
    }

    /// Build the object.
    pub fn build(mut self) -> OObject {
        if !self.camera_samples.is_empty() {
            let mut geom = OProperty::compound(".geom");
            let mut geom_meta = MetaData::new();
            geom_meta.set("schema", "AbcGeom_Light_v1");
            geom.meta_data = geom_meta;

            let mut cam_compound = OProperty::compound(".camera");
            let mut cam_meta = MetaData::new();
            cam_meta.set("schema", "AbcGeom_Camera_v1");
            cam_compound.meta_data = cam_meta;

            let mut core = OProperty::scalar(
                ".core",
                DataType::new(PlainOldDataType::Float64, 16),
            );
            core.time_sampling_index = self.time_sampling_index;

            for sample in &self.camera_samples {
                let props: [f64; 16] = [
                    sample.focal_length,
                    sample.horizontal_aperture,
                    sample.horizontal_film_offset,
                    sample.vertical_aperture,
                    sample.vertical_film_offset,
                    sample.lens_squeeze_ratio,
                    sample.overscan_left,
                    sample.overscan_right,
                    sample.overscan_top,
                    sample.overscan_bottom,
                    sample.f_stop,
                    sample.focus_distance,
                    sample.shutter_open,
                    sample.shutter_close,
                    sample.near_clipping_plane,
                    sample.far_clipping_plane,
                ];
                core.add_scalar_sample(bytemuck::cast_slice(&props));
            }

            if let OPropertyData::Compound(children) = &mut cam_compound.data {
                children.push(core);
            }

            if let OPropertyData::Compound(children) = &mut geom.data {
                children.push(cam_compound);
            }

            self.object.properties.push(geom);
        }

        self.object
    }

    /// Add child object.
    pub fn add_child(&mut self, child: OObject) {
        self.object.children.push(child);
    }
}
