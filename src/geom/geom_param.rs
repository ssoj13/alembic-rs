//! Geometry parameter support for Alembic.
//!
//! IGeomParam provides typed access to geometry attributes like UVs, normals, 
//! colors, etc. These can be either indexed (shared values with index array)
//! or non-indexed (one value per element).

use crate::abc::ICompoundProperty;
use crate::core::{GeometryScope, SampleSelector, PropertyHeader};
use crate::util::{DataType, Result, Error};

/// Metadata key for geometry scope.
pub const GEOM_SCOPE_KEY: &str = "geoScope";
/// Metadata key for array extent.
pub const ARRAY_EXTENT_KEY: &str = "arrayExtent";
/// Metadata key for POD name.
pub const POD_NAME_KEY: &str = "podName";
/// Metadata key for POD extent.
pub const POD_EXTENT_KEY: &str = "podExtent";

/// Name of values sub-property in indexed GeomParam.
pub const VALS_PROPERTY_NAME: &str = ".vals";
/// Name of indices sub-property in indexed GeomParam.
pub const INDICES_PROPERTY_NAME: &str = ".indices";

/// Sample data from a geometry parameter.
#[derive(Clone, Debug)]
pub struct GeomParamSample {
    /// Raw values data.
    pub values: Vec<u8>,
    /// Optional indices for indexed parameters.
    pub indices: Option<Vec<u32>>,
    /// Scope of the data.
    pub scope: GeometryScope,
    /// Whether this is indexed.
    pub is_indexed: bool,
    /// Data type of values.
    pub data_type: DataType,
}

impl Default for GeomParamSample {
    fn default() -> Self {
        Self {
            values: Vec::new(),
            indices: None,
            scope: GeometryScope::Unknown,
            is_indexed: false,
            data_type: DataType::UNKNOWN,
        }
    }
}

impl GeomParamSample {
    /// Check if sample has valid data.
    pub fn is_valid(&self) -> bool {
        !self.values.is_empty()
    }
    
    /// Get number of values (raw count, not expanded).
    pub fn num_values(&self) -> usize {
        if self.data_type.num_bytes() > 0 {
            self.values.len() / self.data_type.num_bytes()
        } else {
            0
        }
    }
    
    /// Get number of indices (for indexed params).
    pub fn num_indices(&self) -> usize {
        self.indices.as_ref().map(|i| i.len()).unwrap_or(0)
    }
    
    /// Get values as f32 slice (for Vec2/Vec3/etc).
    pub fn values_as_f32(&self) -> &[f32] {
        bytemuck::try_cast_slice(&self.values).unwrap_or(&[])
    }
    
    /// Get values as f64 slice.
    pub fn values_as_f64(&self) -> &[f64] {
        bytemuck::try_cast_slice(&self.values).unwrap_or(&[])
    }
    
    /// Get values as i32 slice.
    pub fn values_as_i32(&self) -> &[i32] {
        bytemuck::try_cast_slice(&self.values).unwrap_or(&[])
    }
    
    /// Get values as u32 slice.
    pub fn values_as_u32(&self) -> &[u32] {
        bytemuck::try_cast_slice(&self.values).unwrap_or(&[])
    }
    
    /// Expand indexed values to per-element values.
    /// Returns the expanded f32 data.
    pub fn expand_f32(&self, components: usize) -> Vec<f32> {
        let vals = self.values_as_f32();
        
        if let Some(ref indices) = self.indices {
            // Expand using indices
            let mut result = Vec::with_capacity(indices.len() * components);
            for &idx in indices {
                let start = (idx as usize) * components;
                if start + components <= vals.len() {
                    result.extend_from_slice(&vals[start..start + components]);
                } else {
                    // Out of bounds - fill with zeros
                    result.extend(std::iter::repeat_n(0.0f32, components));
                }
            }
            result
        } else {
            // Not indexed - return as-is
            vals.to_vec()
        }
    }
    
    /// Expand to Vec2 array.
    pub fn expand_vec2(&self) -> Vec<glam::Vec2> {
        let expanded = self.expand_f32(2);
        expanded.chunks_exact(2)
            .map(|c| glam::vec2(c[0], c[1]))
            .collect()
    }
    
    /// Expand to Vec3 array.
    pub fn expand_vec3(&self) -> Vec<glam::Vec3> {
        let expanded = self.expand_f32(3);
        expanded.chunks_exact(3)
            .map(|c| glam::vec3(c[0], c[1], c[2]))
            .collect()
    }
}

/// Input geometry parameter reader.
/// 
/// Handles both indexed and non-indexed geometry parameters.
/// Indexed params have a compound property with `.vals` and `.indices`.
/// Non-indexed params are just an array property.
pub struct IGeomParam<'a> {
    /// Parent compound property.
    parent: &'a ICompoundProperty<'a>,
    /// Property name.
    name: String,
    /// Whether this is indexed.
    is_indexed: bool,
    /// Geometry scope.
    scope: GeometryScope,
    /// Data type.
    data_type: DataType,
}

impl<'a> IGeomParam<'a> {
    /// Create a new IGeomParam from a compound property and parameter name.
    /// Returns None if the parameter doesn't exist.
    pub fn new(parent: &'a ICompoundProperty<'a>, name: &str) -> Option<Self> {
        let prop = parent.property_by_name(name)?;
        let header = prop.header();
        
        // Check if indexed (compound) or non-indexed (array)
        let is_indexed = prop.is_compound();
        
        // Get scope from metadata
        let scope = Self::extract_scope(header);
        
        // Get data type
        let data_type = if is_indexed {
            // For indexed, we need to look at .vals
            if let Some(compound) = prop.as_compound() {
                if let Some(vals_prop) = compound.property_by_name(VALS_PROPERTY_NAME) {
                    vals_prop.header().data_type
                } else {
                    DataType::UNKNOWN
                }
            } else {
                DataType::UNKNOWN
            }
        } else {
            header.data_type
        };
        
        Some(Self {
            parent,
            name: name.to_string(),
            is_indexed,
            scope,
            data_type,
        })
    }
    
    /// Extract geometry scope from property header metadata.
    fn extract_scope(header: &PropertyHeader) -> GeometryScope {
        if let Some(scope_str) = header.meta_data.get(GEOM_SCOPE_KEY) {
            GeometryScope::parse(scope_str)
        } else {
            GeometryScope::Unknown
        }
    }
    
    /// Get the parameter name.
    pub fn name(&self) -> &str {
        &self.name
    }
    
    /// Check if this parameter is indexed.
    pub fn is_indexed(&self) -> bool {
        self.is_indexed
    }
    
    /// Get the geometry scope.
    pub fn scope(&self) -> GeometryScope {
        self.scope
    }
    
    /// Get the data type.
    pub fn data_type(&self) -> DataType {
        self.data_type
    }
    
    /// Get number of samples.
    pub fn num_samples(&self) -> usize {
        let Some(prop) = self.parent.property_by_name(&self.name) else {
            return 0;
        };
        
        if self.is_indexed {
            // For indexed, check both vals and indices
            if let Some(compound) = prop.as_compound() {
                let mut vals_count = 0usize;
                let mut indices_count = 0usize;
                
                if let Some(vals_prop) = compound.property_by_name(VALS_PROPERTY_NAME) {
                    if let Some(array) = vals_prop.as_array() {
                        vals_count = array.num_samples();
                    }
                }
                if let Some(indices_prop) = compound.property_by_name(INDICES_PROPERTY_NAME) {
                    if let Some(array) = indices_prop.as_array() {
                        indices_count = array.num_samples();
                    }
                }
                vals_count.max(indices_count)
            } else {
                0
            }
        } else if let Some(array) = prop.as_array() {
            array.num_samples()
        } else {
            0
        }
    }
    
    /// Check if this parameter is constant (single sample).
    pub fn is_constant(&self) -> bool {
        self.num_samples() <= 1
    }
    
    /// Read a sample at the given index.
    pub fn get_sample(&self, sel: impl Into<SampleSelector>) -> Result<GeomParamSample> {
        let sel = sel.into();
        let index = match sel {
            SampleSelector::Index(i) => i,
            _ => 0,
        };
        
        let prop = self.parent.property_by_name(&self.name)
            .ok_or_else(|| Error::invalid(format!("Property {} not found", self.name)))?;
        
        let mut sample = GeomParamSample {
            scope: self.scope,
            is_indexed: self.is_indexed,
            data_type: self.data_type,
            ..Default::default()
        };
        
        if self.is_indexed {
            // Read from compound with .vals and .indices
            let compound = prop.as_compound()
                .ok_or_else(|| Error::invalid("Expected compound for indexed param"))?;
            
            // Read values
            if let Some(vals_prop) = compound.property_by_name(VALS_PROPERTY_NAME) {
                if let Some(array) = vals_prop.as_array() {
                    sample.values = array.read_sample_vec(index)?;
                }
            }
            
            // Read indices - must be done after vals_prop goes out of scope
            let indices_data: Option<Vec<u8>> = {
                if let Some(indices_prop) = compound.property_by_name(INDICES_PROPERTY_NAME) {
                    if let Some(array) = indices_prop.as_array() {
                        Some(array.read_sample_vec(index)?)
                    } else {
                        None
                    }
                } else {
                    None
                }
            };
            if let Some(data) = indices_data {
                sample.indices = bytemuck::try_cast_slice::<_, u32>(&data).ok().map(|s| s.to_vec());
            }
        } else {
            // Non-indexed - just read the array
            if let Some(array) = prop.as_array() {
                sample.values = array.read_sample_vec(index)?;
            }
        }
        
        Ok(sample)
    }
    
    /// Read sample and expand indexed data to per-element.
    pub fn get_expanded_sample(&self, sel: impl Into<SampleSelector>) -> Result<GeomParamSample> {
        let mut sample = self.get_sample(sel)?;
        
        if sample.is_indexed && sample.indices.is_some() {
            // Expand the values
            let element_size = self.data_type.num_bytes();
            if element_size > 0 {
                let indices = sample.indices.take().unwrap();
                let old_values = std::mem::take(&mut sample.values);
                
                let mut new_values = Vec::with_capacity(indices.len() * element_size);
                for &idx in &indices {
                    let start = (idx as usize) * element_size;
                    if start + element_size <= old_values.len() {
                        new_values.extend_from_slice(&old_values[start..start + element_size]);
                    } else {
                        // Out of bounds - fill with zeros
                        new_values.extend(std::iter::repeat_n(0u8, element_size));
                    }
                }
                sample.values = new_values;
            }
            sample.is_indexed = false;
        }
        
        Ok(sample)
    }
    
    /// Get array extent from metadata (for multi-component values).
    pub fn array_extent(&self) -> usize {
        let Some(prop) = self.parent.property_by_name(&self.name) else {
            return 1;
        };
        
        if let Some(ext_str) = prop.header().meta_data.get(ARRAY_EXTENT_KEY) {
            ext_str.parse().unwrap_or(1)
        } else {
            1
        }
    }
    
    /// Get number of unique values in the sample.
    pub fn num_vals(&self, sel: impl Into<SampleSelector>) -> Result<usize> {
        let sample = self.get_sample(sel)?;
        Ok(sample.num_values())
    }
    
    /// Get number of indices (or elements if non-indexed).
    pub fn num_indices(&self, sel: impl Into<SampleSelector>) -> Result<usize> {
        let sample = self.get_sample(sel)?;
        if sample.is_indexed {
            Ok(sample.num_indices())
        } else {
            Ok(sample.num_values())
        }
    }
    
    /// Check if this param is valid.
    pub fn valid(&self) -> bool {
        self.parent.property_by_name(&self.name).is_some()
    }
    
    /// Get time sampling index.
    pub fn time_sampling_index(&self) -> u32 {
        let Some(prop) = self.parent.property_by_name(&self.name) else {
            return 0;
        };
        
        if self.is_indexed {
            if let Some(compound) = prop.as_compound() {
                // Get from .vals property
                if let Some(vals_prop) = compound.property_by_name(VALS_PROPERTY_NAME) {
                    return vals_prop.header().time_sampling_index;
                }
            }
        }
        prop.header().time_sampling_index
    }
    
    /// Read UVs directly as Vec2 array (convenience for UV params).
    pub fn get_uvs(&self, sel: impl Into<SampleSelector>) -> Result<Vec<glam::Vec2>> {
        let sample = self.get_expanded_sample(sel)?;
        Ok(sample.expand_vec2())
    }
    
    /// Read normals directly as Vec3 array (convenience for normal params).
    pub fn get_normals(&self, sel: impl Into<SampleSelector>) -> Result<Vec<glam::Vec3>> {
        let sample = self.get_expanded_sample(sel)?;
        Ok(sample.expand_vec3())
    }
    
    /// Read colors directly as Vec3 array (convenience for color3 params).
    pub fn get_colors3(&self, sel: impl Into<SampleSelector>) -> Result<Vec<glam::Vec3>> {
        let sample = self.get_expanded_sample(sel)?;
        Ok(sample.expand_vec3())
    }
    
    /// Read colors directly as Vec4 array (convenience for color4 params).
    pub fn get_colors4(&self, sel: impl Into<SampleSelector>) -> Result<Vec<glam::Vec4>> {
        let sample = self.get_expanded_sample(sel)?;
        let expanded = sample.expand_f32(4);
        Ok(expanded.chunks_exact(4)
            .map(|c| glam::vec4(c[0], c[1], c[2], c[3]))
            .collect())
    }
}

// Type aliases for common geometry parameter types

/// Vec2f geometry parameter (UVs).
pub type IV2fGeomParam<'a> = IGeomParam<'a>;

/// Vec3f geometry parameter (normals, colors).
pub type IV3fGeomParam<'a> = IGeomParam<'a>;

/// Normal3f geometry parameter.
pub type IN3fGeomParam<'a> = IGeomParam<'a>;

/// Color3f geometry parameter.
pub type IC3fGeomParam<'a> = IGeomParam<'a>;

/// Color4f geometry parameter.
pub type IC4fGeomParam<'a> = IGeomParam<'a>;

/// Int32 geometry parameter.
pub type IInt32GeomParam<'a> = IGeomParam<'a>;

/// UInt32 geometry parameter.
pub type IUInt32GeomParam<'a> = IGeomParam<'a>;

/// Float geometry parameter.
pub type IFloatGeomParam<'a> = IGeomParam<'a>;

// ============================================================================
// Output Geometry Parameter
// ============================================================================

use crate::core::MetaData;

/// Output geometry parameter sample.
#[derive(Clone, Debug)]
pub struct OGeomParamSample<T> {
    /// Values data.
    pub values: Vec<T>,
    /// Optional indices for indexed parameters.
    pub indices: Option<Vec<u32>>,
    /// Scope of the data.
    pub scope: GeometryScope,
}

impl<T> Default for OGeomParamSample<T> {
    fn default() -> Self {
        Self {
            values: Vec::new(),
            indices: None,
            scope: GeometryScope::Unknown,
        }
    }
}

impl<T> OGeomParamSample<T> {
    /// Create non-indexed sample.
    pub fn new(values: Vec<T>, scope: GeometryScope) -> Self {
        Self { values, indices: None, scope }
    }
    
    /// Create indexed sample.
    pub fn indexed(values: Vec<T>, indices: Vec<u32>, scope: GeometryScope) -> Self {
        Self { values, indices: Some(indices), scope }
    }
    
    /// Check if indexed.
    pub fn is_indexed(&self) -> bool {
        self.indices.is_some()
    }
}

/// Output geometry parameter builder.
/// 
/// Creates GeomParam properties for writing to Alembic files.
/// Supports both indexed and non-indexed parameters.
pub struct OGeomParam {
    /// Property name.
    name: String,
    /// Data type.
    data_type: DataType,
    /// Geometry scope.
    scope: GeometryScope,
    /// Whether indexed.
    is_indexed: bool,
    /// Values samples (raw bytes).
    values_samples: Vec<Vec<u8>>,
    /// Indices samples (for indexed params).
    indices_samples: Vec<Vec<u32>>,
    /// Time sampling index.
    time_sampling_index: u32,
}

impl OGeomParam {
    /// Create a new output geometry parameter.
    pub fn new(name: &str, data_type: DataType, scope: GeometryScope, is_indexed: bool) -> Self {
        Self {
            name: name.to_string(),
            data_type,
            scope,
            is_indexed,
            values_samples: Vec::new(),
            indices_samples: Vec::new(),
            time_sampling_index: 0,
        }
    }
    
    /// Create non-indexed float parameter.
    pub fn float(name: &str, scope: GeometryScope) -> Self {
        Self::new(name, DataType::FLOAT32, scope, false)
    }
    
    /// Create non-indexed Vec2f parameter (UVs).
    pub fn vec2f(name: &str, scope: GeometryScope) -> Self {
        Self::new(name, DataType::VEC2F, scope, false)
    }
    
    /// Create non-indexed Vec3f parameter (normals, colors).
    pub fn vec3f(name: &str, scope: GeometryScope) -> Self {
        Self::new(name, DataType::VEC3F, scope, false)
    }
    
    /// Create indexed Vec2f parameter.
    pub fn vec2f_indexed(name: &str, scope: GeometryScope) -> Self {
        Self::new(name, DataType::VEC2F, scope, true)
    }
    
    /// Create indexed Vec3f parameter.
    pub fn vec3f_indexed(name: &str, scope: GeometryScope) -> Self {
        Self::new(name, DataType::VEC3F, scope, true)
    }
    
    /// Set time sampling index.
    pub fn with_time_sampling(mut self, index: u32) -> Self {
        self.time_sampling_index = index;
        self
    }
    
    /// Add a sample with typed data.
    pub fn add_sample<T: bytemuck::Pod>(&mut self, sample: &OGeomParamSample<T>) {
        self.values_samples.push(bytemuck::try_cast_slice::<_, u8>(&sample.values).unwrap_or(&[]).to_vec());
        if self.is_indexed {
            self.indices_samples.push(sample.indices.clone().unwrap_or_default());
        }
        self.scope = sample.scope;
    }
    
    /// Add raw values sample (non-indexed).
    pub fn add_values<T: bytemuck::Pod>(&mut self, values: &[T]) {
        self.values_samples.push(bytemuck::cast_slice(values).to_vec());
    }
    
    /// Add indexed sample.
    pub fn add_indexed<T: bytemuck::Pod>(&mut self, values: &[T], indices: &[u32]) {
        self.values_samples.push(bytemuck::cast_slice(values).to_vec());
        self.indices_samples.push(indices.to_vec());
    }
    
    /// Get property name.
    pub fn name(&self) -> &str {
        &self.name
    }
    
    /// Check if indexed.
    pub fn is_indexed(&self) -> bool {
        self.is_indexed
    }
    
    /// Get number of samples.
    pub fn num_samples(&self) -> usize {
        self.values_samples.len()
    }
    
    /// Get scope.
    pub fn scope(&self) -> GeometryScope {
        self.scope
    }
    
    /// Get data type.
    pub fn data_type(&self) -> DataType {
        self.data_type
    }
    
    /// Build metadata for this parameter.
    pub fn build_meta_data(&self) -> MetaData {
        let mut meta = MetaData::new();
        meta.set(GEOM_SCOPE_KEY, self.scope.as_str());
        if self.data_type.extent > 1 {
            meta.set(ARRAY_EXTENT_KEY, self.data_type.extent.to_string());
        }
        meta.set(POD_NAME_KEY, self.data_type.pod.name());
        meta.set(POD_EXTENT_KEY, self.data_type.extent.to_string());
        meta
    }
    
    /// Get values for sample index.
    pub fn values(&self, index: usize) -> Option<&[u8]> {
        self.values_samples.get(index).map(|v| v.as_slice())
    }
    
    /// Get indices for sample index (indexed params only).
    pub fn indices(&self, index: usize) -> Option<&[u32]> {
        self.indices_samples.get(index).map(|v| v.as_slice())
    }
    
    /// Get time sampling index.
    pub fn time_sampling_index(&self) -> u32 {
        self.time_sampling_index
    }
}

// Type aliases for output geometry parameters

/// Output Vec2f geometry parameter (UVs).
pub type OV2fGeomParam = OGeomParam;

/// Output Vec3f geometry parameter (normals, colors).
pub type OV3fGeomParam = OGeomParam;

/// Output Normal3f geometry parameter.
pub type ON3fGeomParam = OGeomParam;

/// Output Color3f geometry parameter.
pub type OC3fGeomParam = OGeomParam;

/// Output Color4f geometry parameter.
pub type OC4fGeomParam = OGeomParam;

/// Output Int32 geometry parameter.
pub type OInt32GeomParam = OGeomParam;

/// Output UInt32 geometry parameter.
pub type OUInt32GeomParam = OGeomParam;

/// Output Float geometry parameter.
pub type OFloatGeomParam = OGeomParam;

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_geom_param_sample_default() {
        let sample = GeomParamSample::default();
        assert!(!sample.is_valid());
        assert_eq!(sample.scope, GeometryScope::Unknown);
        assert!(!sample.is_indexed);
    }
    
    #[test]
    fn test_geom_param_sample_expand() {
        let sample = GeomParamSample {
            values: bytemuck::cast_slice::<_, u8>(&[0.0f32, 1.0, 2.0, 3.0, 4.0, 5.0]).to_vec(),
            indices: Some(vec![1, 0, 1, 0]),
            scope: GeometryScope::FaceVarying,
            is_indexed: true,
            data_type: DataType::FLOAT32,
        };
        
        // Expand with 1 component
        let expanded = sample.expand_f32(1);
        // indices [1,0,1,0] -> values [1.0, 0.0, 1.0, 0.0]
        assert_eq!(expanded.len(), 4);
        assert_eq!(expanded[0], 1.0);
        assert_eq!(expanded[1], 0.0);
    }
    
    #[test]
    fn test_geom_param_sample_expand_vec2() {
        let sample = GeomParamSample {
            values: bytemuck::cast_slice::<_, u8>(&[0.0f32, 0.0, 1.0, 1.0, 0.5, 0.5]).to_vec(),
            indices: Some(vec![2, 0, 1]),
            scope: GeometryScope::FaceVarying,
            is_indexed: true,
            data_type: DataType::VEC2F,
        };
        
        let vecs = sample.expand_vec2();
        assert_eq!(vecs.len(), 3);
        assert_eq!(vecs[0], glam::vec2(0.5, 0.5)); // index 2
        assert_eq!(vecs[1], glam::vec2(0.0, 0.0)); // index 0
        assert_eq!(vecs[2], glam::vec2(1.0, 1.0)); // index 1
    }
}
