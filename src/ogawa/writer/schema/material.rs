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

/// Material sample data for output.
pub struct OMaterialSample {
    pub targets: Vec<String>,
    pub shader_types: HashMap<String, Vec<String>>,
    pub shader_names: HashMap<(String, String), String>,
    pub params: Vec<ShaderParam>,
}

impl OMaterialSample {
    /// Create empty material sample.
    pub fn new() -> Self {
        Self {
            targets: Vec::new(),
            shader_types: HashMap::new(),
            shader_names: HashMap::new(),
            params: Vec::new(),
        }
    }

    /// Add a shader.
    pub fn add_shader(&mut self, target: &str, shader_type: &str, shader_name: &str) {
        if !self.targets.contains(&target.to_string()) {
            self.targets.push(target.to_string());
        }
        self.shader_types.entry(target.to_string()).or_default().push(shader_type.to_string());
        self.shader_names.insert(
            (target.to_string(), shader_type.to_string()),
            shader_name.to_string(),
        );
    }

    /// Add a parameter.
    pub fn add_param(&mut self, param: ShaderParam) {
        self.params.push(param);
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

        if !self.sample.targets.is_empty() {
            let targets_str = self.sample.targets.join(";");
            let mut targets_prop = OProperty::scalar(
                ".targets",
                DataType::new(PlainOldDataType::String, 1),
            );
            targets_prop.add_scalar_sample(targets_str.as_bytes());

            if let OPropertyData::Compound(children) = &mut mat.data {
                children.push(targets_prop);

                for target in &self.sample.targets {
                    if let Some(types) = self.sample.shader_types.get(target) {
                        let types_str = types.join(";");
                        let mut types_prop = OProperty::scalar(
                            &format!(".{}.shaderTypes", target),
                            DataType::new(PlainOldDataType::String, 1),
                        );
                        types_prop.add_scalar_sample(types_str.as_bytes());
                        children.push(types_prop);

                        for shader_type in types {
                            if let Some(name) = self
                                .sample
                                .shader_names
                                .get(&(target.clone(), shader_type.clone()))
                            {
                                let mut name_prop = OProperty::scalar(
                                    &format!(".{}.{}.shaderName", target, shader_type),
                                    DataType::new(PlainOldDataType::String, 1),
                                );
                                name_prop.add_scalar_sample(name.as_bytes());
                                children.push(name_prop);
                            }
                        }
                    }
                }
            }
        }

        for param in &self.sample.params {
            let prop_name = format!(".params.{}", param.name);
            let (dt, data) = match &param.value {
                ShaderParamValue::Float(v) => (
                    DataType::new(PlainOldDataType::Float32, 1),
                    bytemuck::bytes_of(v).to_vec(),
                ),
                ShaderParamValue::Double(v) => (
                    DataType::new(PlainOldDataType::Float64, 1),
                    bytemuck::bytes_of(v).to_vec(),
                ),
                ShaderParamValue::Vec2(v) => (
                    DataType::new(PlainOldDataType::Float32, 2),
                    bytemuck::bytes_of(v).to_vec(),
                ),
                ShaderParamValue::Vec3(v) | ShaderParamValue::Color3(v) => (
                    DataType::new(PlainOldDataType::Float32, 3),
                    bytemuck::bytes_of(v).to_vec(),
                ),
                ShaderParamValue::Vec4(v) | ShaderParamValue::Color4(v) => (
                    DataType::new(PlainOldDataType::Float32, 4),
                    bytemuck::bytes_of(v).to_vec(),
                ),
                ShaderParamValue::Matrix(m) => (
                    DataType::new(PlainOldDataType::Float32, 16),
                    bytemuck::bytes_of(m).to_vec(),
                ),
                ShaderParamValue::Int(v) => (
                    DataType::new(PlainOldDataType::Int32, 1),
                    bytemuck::bytes_of(v).to_vec(),
                ),
                ShaderParamValue::String(s) => (
                    DataType::new(PlainOldDataType::String, 1),
                    s.as_bytes().to_vec(),
                ),
                ShaderParamValue::Bool(v) => (
                    DataType::new(PlainOldDataType::Boolean, 1),
                    vec![*v as u8],
                ),
                ShaderParamValue::FloatArray(arr) => (
                    DataType::new(PlainOldDataType::Float32, 1),
                    bytemuck::cast_slice(arr).to_vec(),
                ),
                ShaderParamValue::IntArray(arr) => (
                    DataType::new(PlainOldDataType::Int32, 1),
                    bytemuck::cast_slice(arr).to_vec(),
                ),
                ShaderParamValue::StringArray(arr) => (
                    DataType::new(PlainOldDataType::String, 1),
                    arr.join(";").as_bytes().to_vec(),
                ),
            };

            let mut prop = OProperty::scalar(&prop_name, dt);
            prop.add_scalar_sample(&data);

            if let OPropertyData::Compound(children) = &mut mat.data {
                children.push(prop);
            }
        }

        self.object.properties.push(mat);
        self.object
    }
}
