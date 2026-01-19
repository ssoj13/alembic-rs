//! Material schema writer.
//!
//! References:
//! - `_ref/alembic/lib/Alembic/AbcMaterial/OMaterial.cpp`
//! - `_ref/alembic/lib/Alembic/AbcMaterial/OMaterial.h`

use std::collections::HashMap;

use crate::core::MetaData;
use crate::material::{ShaderParam, ShaderParamValue};
use crate::util::{DataType, PlainOldDataType};

use super::super::object::OObject;
use super::super::property::{OProperty, OPropertyData};
use super::super::write_util::encode_string_array;

/// Material sample data for output.
pub struct OMaterialSample {
    pub shader_names: HashMap<(String, String), String>,
    pub params: HashMap<(String, String), Vec<ShaderParam>>,
}

impl OMaterialSample {
    /// Create empty material sample.
    pub fn new() -> Self {
        Self {
            shader_names: HashMap::new(),
            params: HashMap::new(),
        }
    }

    /// Add a shader.
    pub fn add_shader(&mut self, target: &str, shader_type: &str, shader_name: &str) {
        self.shader_names.insert(
            (target.to_string(), shader_type.to_string()),
            shader_name.to_string(),
        );
    }

    /// Add a parameter.
    pub fn add_param(&mut self, target: &str, shader_type: &str, param: ShaderParam) {
        self.params
            .entry((target.to_string(), shader_type.to_string()))
            .or_default()
            .push(param);
    }
}

impl Default for OMaterialSample {
    fn default() -> Self {
        Self::new()
    }
}

/// Material schema writer.
pub struct OMaterial {
    object: OObject,
    sample: OMaterialSample,
}

impl OMaterial {
    /// Create new Material.
    pub fn new(name: &str) -> Self {
        let mut object = OObject::new(name);
        let mut meta = MetaData::new();
        meta.set("schema", "AbcMaterial_Material_v1");
        meta.set("schemaObjTitle", "AbcMaterial_Material_v1:.material");
        object.meta_data = meta;

        Self { object, sample: OMaterialSample::new() }
    }

    /// Set sample data.
    pub fn set_sample(&mut self, sample: OMaterialSample) {
        self.sample = sample;
    }

    /// Add a shader.
    pub fn add_shader(&mut self, target: &str, shader_type: &str, shader_name: &str) {
        self.sample.add_shader(target, shader_type, shader_name);
    }

    /// Build the object.
    pub fn build(mut self) -> OObject {
        let mut mat = OProperty::compound(".material");
        let mut mat_meta = MetaData::new();
        mat_meta.set("schema", "AbcMaterial_Material_v1");
        mat.meta_data = mat_meta;

        if !self.sample.shader_names.is_empty() {
            let mut entries: Vec<((String, String), String)> = self
                .sample
                .shader_names
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            entries.sort_by(|a, b| a.0.cmp(&b.0));

            let mut strings = Vec::with_capacity(entries.len() * 2);
            for ((target, shader_type), shader_name) in entries {
                strings.push(format!("{}.{}", target, shader_type));
                strings.push(shader_name);
            }

            let data = encode_string_array(&strings);
            let mut shader_names_prop = OProperty::array(
                ".shaderNames",
                DataType::new(PlainOldDataType::String, 1),
            );
            shader_names_prop.add_array_sample(&data, &[strings.len()]);

            if let OPropertyData::Compound(children) = &mut mat.data {
                children.push(shader_names_prop);
            }
        }

        let mut params_entries: Vec<((String, String), Vec<ShaderParam>)> = self
            .sample
            .params
            .into_iter()
            .collect();
        params_entries.sort_by(|a, b| a.0.cmp(&b.0));

        for ((target, shader_type), mut params) in params_entries {
            params.sort_by(|a, b| a.name.cmp(&b.name));
            let mut params_prop = OProperty::compound(&format!("{}.{}.params", target, shader_type));
            if let OPropertyData::Compound(children) = &mut params_prop.data {
                for param in &params {
                    let (dt, data, dims) = match &param.value {
                        ShaderParamValue::Float(v) => (
                            DataType::new(PlainOldDataType::Float32, 1),
                            bytemuck::bytes_of(v).to_vec(),
                            Vec::new(),
                        ),
                        ShaderParamValue::Double(v) => (
                            DataType::new(PlainOldDataType::Float64, 1),
                            bytemuck::bytes_of(v).to_vec(),
                            Vec::new(),
                        ),
                        ShaderParamValue::Vec2(v) => (
                            DataType::new(PlainOldDataType::Float32, 2),
                            bytemuck::bytes_of(v).to_vec(),
                            Vec::new(),
                        ),
                        ShaderParamValue::Vec3(v) | ShaderParamValue::Color3(v) => (
                            DataType::new(PlainOldDataType::Float32, 3),
                            bytemuck::bytes_of(v).to_vec(),
                            Vec::new(),
                        ),
                        ShaderParamValue::Vec4(v) | ShaderParamValue::Color4(v) => (
                            DataType::new(PlainOldDataType::Float32, 4),
                            bytemuck::bytes_of(v).to_vec(),
                            Vec::new(),
                        ),
                        ShaderParamValue::Matrix(m) => (
                            DataType::new(PlainOldDataType::Float32, 16),
                            bytemuck::bytes_of(m).to_vec(),
                            Vec::new(),
                        ),
                        ShaderParamValue::Int(v) => (
                            DataType::new(PlainOldDataType::Int32, 1),
                            bytemuck::bytes_of(v).to_vec(),
                            Vec::new(),
                        ),
                        ShaderParamValue::String(s) => (
                            DataType::new(PlainOldDataType::String, 1),
                            {
                                let mut data = s.as_bytes().to_vec();
                                data.push(0);
                                data
                            },
                            Vec::new(),
                        ),
                        ShaderParamValue::Bool(v) => (
                            DataType::new(PlainOldDataType::Boolean, 1),
                            vec![*v as u8],
                            Vec::new(),
                        ),
                        ShaderParamValue::FloatArray(arr) => (
                            DataType::new(PlainOldDataType::Float32, 1),
                            bytemuck::cast_slice(arr).to_vec(),
                            vec![arr.len()],
                        ),
                        ShaderParamValue::IntArray(arr) => (
                            DataType::new(PlainOldDataType::Int32, 1),
                            bytemuck::cast_slice(arr).to_vec(),
                            vec![arr.len()],
                        ),
                        ShaderParamValue::StringArray(arr) => (
                            DataType::new(PlainOldDataType::String, 1),
                            encode_string_array(arr),
                            vec![arr.len()],
                        ),
                    };

                    let prop = if dims.is_empty() {
                        let mut prop = OProperty::scalar(&param.name, dt);
                        prop.add_scalar_sample(&data);
                        prop
                    } else {
                        let mut prop = OProperty::array(&param.name, dt);
                        prop.add_array_sample(&data, &dims);
                        prop
                    };
                    children.push(prop);
                }
            }
            if let OPropertyData::Compound(children) = &mut mat.data {
                children.push(params_prop);
            }
        }

        self.object.properties.push(mat);
        self.object
    }
}
