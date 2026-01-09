//! Material schema implementation.
//!
//! Provides reading of material data from Alembic files.

use std::collections::HashMap;

use crate::abc::IObject;
use super::MATERIAL_SCHEMA;

/// Shader parameter value.
#[derive(Clone, Debug)]
pub enum ShaderParamValue {
    /// Boolean value.
    Bool(bool),
    /// Integer value.
    Int(i32),
    /// Float value.
    Float(f32),
    /// Double value.
    Double(f64),
    /// String value.
    String(String),
    /// Vec2 value.
    Vec2(glam::Vec2),
    /// Vec3 value.
    Vec3(glam::Vec3),
    /// Vec4 value.
    Vec4(glam::Vec4),
    /// Color3 value (RGB).
    Color3(glam::Vec3),
    /// Color4 value (RGBA).
    Color4(glam::Vec4),
    /// Matrix value.
    Matrix(glam::Mat4),
    /// Array of floats.
    FloatArray(Vec<f32>),
    /// Array of integers.
    IntArray(Vec<i32>),
    /// Array of strings.
    StringArray(Vec<String>),
}

/// Shader parameter with name and value.
#[derive(Clone, Debug)]
pub struct ShaderParam {
    /// Parameter name.
    pub name: String,
    /// Parameter value.
    pub value: ShaderParamValue,
}

impl ShaderParam {
    /// Create a new shader parameter.
    pub fn new(name: &str, value: ShaderParamValue) -> Self {
        Self {
            name: name.to_string(),
            value,
        }
    }
    
    /// Get as float if possible.
    pub fn as_float(&self) -> Option<f32> {
        match &self.value {
            ShaderParamValue::Float(v) => Some(*v),
            ShaderParamValue::Double(v) => Some(*v as f32),
            ShaderParamValue::Int(v) => Some(*v as f32),
            _ => None,
        }
    }
    
    /// Get as string if possible.
    pub fn as_string(&self) -> Option<&str> {
        match &self.value {
            ShaderParamValue::String(s) => Some(s),
            _ => None,
        }
    }
    
    /// Get as vec3 if possible.
    pub fn as_vec3(&self) -> Option<glam::Vec3> {
        match &self.value {
            ShaderParamValue::Vec3(v) => Some(*v),
            ShaderParamValue::Color3(v) => Some(*v),
            _ => None,
        }
    }
}

/// Shader node in a shader network.
#[derive(Clone, Debug)]
pub struct ShaderNode {
    /// Node name.
    pub name: String,
    /// Shader type (e.g., "standard_surface", "image").
    pub shader_type: String,
    /// Target renderer (e.g., "arnold", "renderman").
    pub target: String,
    /// Shader parameters.
    pub parameters: Vec<ShaderParam>,
    /// Input connections (param_name -> (source_node, source_output)).
    pub connections: HashMap<String, (String, String)>,
}

impl ShaderNode {
    /// Create a new shader node.
    pub fn new(name: &str, shader_type: &str, target: &str) -> Self {
        Self {
            name: name.to_string(),
            shader_type: shader_type.to_string(),
            target: target.to_string(),
            parameters: Vec::new(),
            connections: HashMap::new(),
        }
    }
    
    /// Add a parameter.
    pub fn add_param(&mut self, param: ShaderParam) {
        self.parameters.push(param);
    }
    
    /// Get a parameter by name.
    pub fn param(&self, name: &str) -> Option<&ShaderParam> {
        self.parameters.iter().find(|p| p.name == name)
    }
    
    /// Connect an input to another node's output.
    pub fn connect(&mut self, input_name: &str, source_node: &str, source_output: &str) {
        self.connections.insert(
            input_name.to_string(),
            (source_node.to_string(), source_output.to_string()),
        );
    }
    
    /// Check if a parameter is connected.
    pub fn is_connected(&self, param_name: &str) -> bool {
        self.connections.contains_key(param_name)
    }
}

/// Shader network containing interconnected shader nodes.
#[derive(Clone, Debug, Default)]
pub struct ShaderNetwork {
    /// All nodes in the network.
    pub nodes: HashMap<String, ShaderNode>,
    /// Terminal node names for each output type.
    pub terminals: HashMap<String, String>,
}

impl ShaderNetwork {
    /// Create an empty shader network.
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Add a node to the network.
    pub fn add_node(&mut self, node: ShaderNode) {
        self.nodes.insert(node.name.clone(), node);
    }
    
    /// Get a node by name.
    pub fn node(&self, name: &str) -> Option<&ShaderNode> {
        self.nodes.get(name)
    }
    
    /// Get mutable node by name.
    pub fn node_mut(&mut self, name: &str) -> Option<&mut ShaderNode> {
        self.nodes.get_mut(name)
    }
    
    /// Set a terminal node for an output type.
    pub fn set_terminal(&mut self, output_type: &str, node_name: &str) {
        self.terminals.insert(output_type.to_string(), node_name.to_string());
    }
    
    /// Get the terminal node for an output type.
    pub fn terminal(&self, output_type: &str) -> Option<&str> {
        self.terminals.get(output_type).map(|s| s.as_str())
    }
    
    /// Get all node names.
    pub fn node_names(&self) -> Vec<&str> {
        self.nodes.keys().map(|s| s.as_str()).collect()
    }
    
    /// Check if the network is empty.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}

/// Material sample data.
#[derive(Clone, Debug, Default)]
pub struct MaterialSample {
    /// Shader networks per target (e.g., "arnold", "renderman").
    pub networks: HashMap<String, ShaderNetwork>,
    /// Interface parameters (exposed parameters).
    pub interface_params: Vec<ShaderParam>,
}

impl MaterialSample {
    /// Create an empty material sample.
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Get network for a target.
    pub fn network(&self, target: &str) -> Option<&ShaderNetwork> {
        self.networks.get(target)
    }
    
    /// Get all target names.
    pub fn target_names(&self) -> Vec<&str> {
        self.networks.keys().map(|s| s.as_str()).collect()
    }
    
    /// Check if the material has any networks.
    pub fn is_empty(&self) -> bool {
        self.networks.is_empty()
    }
}

/// Input material schema reader.
pub struct IMaterial<'a> {
    object: &'a IObject<'a>,
}

impl<'a> IMaterial<'a> {
    /// Wrap an IObject as IMaterial.
    /// Returns None if the object doesn't have the Material schema.
    pub fn new(object: &'a IObject<'a>) -> Option<Self> {
        if object.matches_schema(MATERIAL_SCHEMA) {
            Some(Self { object })
        } else {
            None
        }
    }
    
    /// Get the underlying object.
    pub fn object(&self) -> &IObject<'a> {
        self.object
    }
    
    /// Get the object name.
    pub fn name(&self) -> &str {
        self.object.name()
    }
    
    /// Get the full path.
    pub fn full_name(&self) -> &str {
        self.object.full_name()
    }
    
    /// Get target names (renderer targets like "arnold", "renderman").
    pub fn target_names(&self) -> Vec<String> {
        let props = self.object.properties();
        let Some(mat_prop) = props.property_by_name(".material") else {
            return Vec::new();
        };
        let Some(mat) = mat_prop.as_compound() else {
            return Vec::new();
        };
        
        // Filter property names that look like targets (not starting with '.')
        mat.property_names()
            .into_iter()
            .filter(|n| !n.starts_with('.'))
            .collect()
    }
    
    /// Get shader type names for a target.
    pub fn shader_type_names(&self, target: &str) -> Vec<String> {
        let props = self.object.properties();
        let Some(mat_prop) = props.property_by_name(".material") else {
            return Vec::new();
        };
        let Some(mat) = mat_prop.as_compound() else {
            return Vec::new();
        };
        let Some(target_prop) = mat.property_by_name(target) else {
            return Vec::new();
        };
        let Some(target_compound) = target_prop.as_compound() else {
            return Vec::new();
        };
        
        target_compound.property_names()
    }
    
    /// Get shader name for a target and shader type.
    pub fn shader(&self, target: &str, shader_type: &str) -> Option<String> {
        let props = self.object.properties();
        let mat_prop = props.property_by_name(".material")?;
        let mat = mat_prop.as_compound()?;
        let target_prop = mat.property_by_name(target)?;
        let target_compound = target_prop.as_compound()?;
        let shader_prop = target_compound.property_by_name(shader_type)?;
        let scalar = shader_prop.as_scalar()?;
        
        // Read shader name as string
        let mut buf = [0u8; 256];
        scalar.read_sample(0, &mut buf).ok()?;
        
        // Find null terminator
        let len = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
        String::from_utf8(buf[..len].to_vec()).ok()
    }
    
    /// Check if this material is valid.
    #[inline]
    pub fn valid(&self) -> bool {
        true
    }
}

// ============================================================================
// Material Assignment Utilities
// ============================================================================

/// Get material assignment path from an object.
/// 
/// Returns the path to the assigned material, if any.
pub fn get_material_assignment(object: &IObject) -> Option<String> {
    let props = object.properties();
    
    // Look for .material.assign property
    let mat_prop = props.property_by_name(".material")?;
    let mat = mat_prop.as_compound()?;
    let assign_prop = mat.property_by_name("assign")?;
    let scalar = assign_prop.as_scalar()?;
    
    // Read assignment path
    let mut buf = [0u8; 512];
    scalar.read_sample(0, &mut buf).ok()?;
    
    let len = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    String::from_utf8(buf[..len].to_vec()).ok()
}

/// Check if an object has material assignments.
pub fn has_material_assignment(object: &IObject) -> bool {
    let props = object.properties();
    
    if let Some(mat_prop) = props.property_by_name(".material") {
        if let Some(mat) = mat_prop.as_compound() {
            return mat.has_property("assign");
        }
    }
    false
}

/// Get face-set material assignments from a mesh.
/// 
/// Returns a map of face-set name to material path.
pub fn get_faceset_material_assignments(object: &IObject) -> HashMap<String, String> {
    let mut assignments = HashMap::new();
    
    // Iterate through child objects looking for FaceSets with material assignments
    for i in 0..object.num_children() {
        if let Some(child) = object.child(i) {
            if child.matches_schema("AbcGeom_FaceSet_v1") {
                if let Some(path) = get_material_assignment(&child) {
                    assignments.insert(child.name().to_string(), path);
                }
            }
        }
    }
    
    assignments
}

// ============================================================================
// Material Flattening
// ============================================================================

/// Flattened material representation.
/// 
/// Contains all resolved shader networks and parameters after flattening
/// inheritance hierarchy.
#[derive(Clone, Debug, Default)]
pub struct FlattenedMaterial {
    /// Fully resolved shader networks per target.
    pub networks: HashMap<String, ShaderNetwork>,
    /// All interface parameters (from this material and inherited).
    pub interface_params: Vec<ShaderParam>,
    /// Source material paths in inheritance order (root first).
    pub inheritance_chain: Vec<String>,
}

impl FlattenedMaterial {
    /// Create an empty flattened material.
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Get network for a target.
    pub fn network(&self, target: &str) -> Option<&ShaderNetwork> {
        self.networks.get(target)
    }
    
    /// Get all target names.
    pub fn target_names(&self) -> Vec<&str> {
        self.networks.keys().map(|s| s.as_str()).collect()
    }
    
    /// Get interface parameter by name.
    pub fn interface_param(&self, name: &str) -> Option<&ShaderParam> {
        self.interface_params.iter().find(|p| p.name == name)
    }
    
    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.networks.is_empty()
    }
}

impl IMaterial<'_> {
    /// Flatten this material into a single representation.
    /// 
    /// Collects all shader networks and parameters from this material.
    /// Note: For full inheritance resolution, use flatten_material() with archive root.
    pub fn flatten(&self) -> FlattenedMaterial {
        let mut result = FlattenedMaterial::new();
        result.inheritance_chain.push(self.full_name().to_string());
        
        // Collect shader networks for each target
        for target in self.target_names() {
            let mut network = ShaderNetwork::new();
            
            for shader_type in self.shader_type_names(&target) {
                if let Some(shader_name) = self.shader(&target, &shader_type) {
                    let node = ShaderNode::new(&shader_type, &shader_name, &target);
                    network.add_node(node);
                    network.set_terminal(&shader_type, &shader_type);
                }
            }
            
            if !network.is_empty() {
                result.networks.insert(target, network);
            }
        }
        
        result
    }
    
    /// Get the inherits path if this material inherits from another.
    pub fn inherits_path(&self) -> Option<String> {
        let props = self.object.properties();
        let mat_prop = props.property_by_name(".material")?;
        let mat = mat_prop.as_compound()?;
        let inherits_prop = mat.property_by_name(".inherits")?;
        let scalar = inherits_prop.as_scalar()?;
        
        let mut buf = [0u8; 512];
        scalar.read_sample(0, &mut buf).ok()?;
        
        let len = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
        String::from_utf8(buf[..len].to_vec()).ok()
    }
    
    /// Check if this material inherits from another.
    pub fn has_inheritance(&self) -> bool {
        self.inherits_path().is_some()
    }
}

/// Merge two flattened materials.
/// Child values take precedence over parent.
pub fn merge_flattened_materials(child: &mut FlattenedMaterial, parent: &FlattenedMaterial) {
    // Prepend parent inheritance chain
    for path in parent.inheritance_chain.iter().rev() {
        if !child.inheritance_chain.contains(path) {
            child.inheritance_chain.insert(0, path.clone());
        }
    }
    
    // Merge networks - parent networks are base, child overrides
    for (target, parent_network) in &parent.networks {
        let child_network = child.networks.entry(target.clone()).or_default();
        
        // Add parent nodes that don't exist in child
        for (node_name, parent_node) in &parent_network.nodes {
            if !child_network.nodes.contains_key(node_name) {
                child_network.nodes.insert(node_name.clone(), parent_node.clone());
            }
        }
        
        // Add parent terminals that don't exist in child
        for (output_type, node_name) in &parent_network.terminals {
            if !child_network.terminals.contains_key(output_type) {
                child_network.terminals.insert(output_type.clone(), node_name.clone());
            }
        }
    }
    
    // Merge interface params - child overrides parent
    for parent_param in &parent.interface_params {
        if !child.interface_params.iter().any(|p| p.name == parent_param.name) {
            child.interface_params.push(parent_param.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_shader_param_conversion() {
        let param = ShaderParam::new("roughness", ShaderParamValue::Float(0.5));
        assert_eq!(param.as_float(), Some(0.5));
        assert_eq!(param.as_string(), None);
        
        let param = ShaderParam::new("name", ShaderParamValue::String("metal".to_string()));
        assert_eq!(param.as_string(), Some("metal"));
        assert_eq!(param.as_float(), None);
    }
    
    #[test]
    fn test_shader_network() {
        let mut network = ShaderNetwork::new();
        
        let mut surface = ShaderNode::new("surface1", "standard_surface", "arnold");
        surface.add_param(ShaderParam::new("base", ShaderParamValue::Float(1.0)));
        surface.add_param(ShaderParam::new("base_color", ShaderParamValue::Color3(glam::vec3(0.8, 0.2, 0.1))));
        
        let texture = ShaderNode::new("texture1", "image", "arnold");
        
        surface.connect("base_color", "texture1", "out_color");
        
        network.add_node(surface);
        network.add_node(texture);
        network.set_terminal("surface", "surface1");
        
        assert_eq!(network.node_names().len(), 2);
        assert_eq!(network.terminal("surface"), Some("surface1"));
        
        let surface_node = network.node("surface1").unwrap();
        assert!(surface_node.is_connected("base_color"));
        assert!(!surface_node.is_connected("base"));
    }
    
    #[test]
    fn test_material_sample() {
        let sample = MaterialSample::new();
        assert!(sample.is_empty());
        assert!(sample.target_names().is_empty());
    }
}
