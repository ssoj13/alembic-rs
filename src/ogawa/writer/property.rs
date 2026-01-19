//! Ogawa property writer types.
//!
//! References:
//! - `_ref/alembic/lib/Alembic/AbcCoreOgawa/SpwImpl.cpp`
//! - `_ref/alembic/lib/Alembic/AbcCoreOgawa/ApwImpl.cpp`
//! - `_ref/alembic/lib/Alembic/AbcCoreOgawa/CpwData.cpp`

use crate::core::{MetaData, SampleDigest};
use crate::util::DataType;

/// Sample with optional pre-computed digest for raw copying.
#[derive(Clone)]
pub struct SampleWithDigest {
    /// Sample data.
    pub data: Vec<u8>,
    /// Optional pre-computed digest (16 bytes). If None, digest is computed.
    pub digest: Option<SampleDigest>,
}

impl SampleWithDigest {
    /// Create sample without digest (will be computed).
    pub fn new(data: Vec<u8>) -> Self {
        Self { data, digest: None }
    }

    /// Create sample with pre-computed digest (for raw copy).
    pub fn with_digest(data: Vec<u8>, digest: SampleDigest) -> Self {
        Self { data, digest: Some(digest) }
    }
}

/// Array sample with optional pre-computed digest.
#[derive(Clone)]
pub struct ArraySampleWithDigest {
    /// Sample data.
    pub data: Vec<u8>,
    /// Dimensions.
    pub dims: Vec<usize>,
    /// Optional pre-computed digest (16 bytes). If None, digest is computed.
    pub digest: Option<SampleDigest>,
}

impl ArraySampleWithDigest {
    /// Create sample without digest (will be computed).
    pub fn new(data: Vec<u8>, dims: Vec<usize>) -> Self {
        Self { data, dims, digest: None }
    }

    /// Create sample with pre-computed digest (for raw copy).
    pub fn with_digest(data: Vec<u8>, dims: Vec<usize>, digest: SampleDigest) -> Self {
        Self { data, dims, digest: Some(digest) }
    }
}

/// Property data variants.
#[derive(Clone)]
pub enum OPropertyData {
    /// Scalar property samples with optional digests.
    Scalar(Vec<SampleWithDigest>),
    /// Array property samples with optional digests.
    Array(Vec<ArraySampleWithDigest>),
    /// Compound property children.
    Compound(Vec<OProperty>),
}

/// Property for writing.
#[derive(Clone)]
pub struct OProperty {
    /// Property name.
    pub name: String,
    /// Data type.
    pub data_type: DataType,
    /// Metadata.
    pub meta_data: MetaData,
    /// Time sampling index.
    pub time_sampling_index: u32,
    /// First changed sample index.
    pub first_changed_index: u32,
    /// Last changed sample index.
    pub last_changed_index: u32,
    /// Property data.
    pub data: OPropertyData,
    /// Is scalar-like (for array properties that behave like scalars).
    /// When true, bit 0 of property type is set (ptype 3 instead of 2 for arrays).
    pub is_scalar_like: bool,
    /// Data write order - determines order of data in file (C++ parity).
    /// Lower values are written first. Properties with same order use compound order.
    pub data_write_order: u32,
}

impl OProperty {
    /// Create a scalar property.
    pub fn scalar(name: &str, data_type: DataType) -> Self {
        Self {
            name: name.to_string(),
            data_type,
            meta_data: MetaData::new(),
            time_sampling_index: 0,
            first_changed_index: 0,
            last_changed_index: 0,
            data: OPropertyData::Scalar(Vec::new()),
            is_scalar_like: true,
            data_write_order: u32::MAX, // Default: use compound order
        }
    }

    /// Create an array property.
    pub fn array(name: &str, data_type: DataType) -> Self {
        Self {
            name: name.to_string(),
            data_type,
            meta_data: MetaData::new(),
            time_sampling_index: 0,
            first_changed_index: 0,
            last_changed_index: 0,
            data: OPropertyData::Array(Vec::new()),
            is_scalar_like: true,
            data_write_order: u32::MAX,
        }
    }

    /// Create an array property that behaves like a scalar (scalar-like).
    pub fn scalar_like_array(name: &str, data_type: DataType) -> Self {
        Self {
            name: name.to_string(),
            data_type,
            meta_data: MetaData::new(),
            time_sampling_index: 0,
            first_changed_index: 0,
            last_changed_index: 0,
            data: OPropertyData::Array(Vec::new()),
            is_scalar_like: true,
            data_write_order: u32::MAX,
        }
    }

    /// Create a compound property.
    pub fn compound(name: &str) -> Self {
        Self {
            name: name.to_string(),
            data_type: DataType::UNKNOWN,
            meta_data: MetaData::new(),
            time_sampling_index: 0,
            first_changed_index: 0,
            last_changed_index: 0,
            data: OPropertyData::Compound(Vec::new()),
            is_scalar_like: false,
            data_write_order: u32::MAX,
        }
    }

    /// Set metadata.
    pub fn with_meta_data(mut self, md: MetaData) -> Self {
        self.meta_data = md;
        self
    }

    /// Set time sampling index.
    pub fn with_time_sampling(mut self, index: u32) -> Self {
        self.time_sampling_index = index;
        self
    }

    /// Add a scalar sample.
    pub fn add_scalar_sample(&mut self, data: &[u8]) {
        if let OPropertyData::Scalar(samples) = &mut self.data {
            let sample_index = samples.len() as u32;
            let is_same = samples.last().map_or(false, |prev| prev.data == data);
            samples.push(SampleWithDigest::new(data.to_vec()));
            if sample_index == 0 {
                self.first_changed_index = 0;
                self.last_changed_index = 0;
            } else if !is_same {
                if self.first_changed_index == 0 {
                    self.first_changed_index = sample_index;
                }
                self.last_changed_index = sample_index;
            }
        }
    }

    /// Add a scalar sample with pre-computed digest (for raw copy).
    pub fn add_scalar_sample_with_digest(&mut self, data: &[u8], digest: SampleDigest) {
        if let OPropertyData::Scalar(samples) = &mut self.data {
            let sample_index = samples.len() as u32;
            let is_same = samples.last().map_or(false, |prev| prev.data == data);
            samples.push(SampleWithDigest::with_digest(data.to_vec(), digest));
            if sample_index == 0 {
                self.first_changed_index = 0;
                self.last_changed_index = 0;
            } else if !is_same {
                if self.first_changed_index == 0 {
                    self.first_changed_index = sample_index;
                }
                self.last_changed_index = sample_index;
            }
        }
    }

    /// Add a scalar sample from Pod type.
    pub fn add_scalar_pod<T: bytemuck::Pod>(&mut self, value: &T) {
        self.add_scalar_sample(bytemuck::bytes_of(value));
    }

    /// Add a scalar string sample (auto null-terminated).
    pub fn add_scalar_string(&mut self, s: &str) {
        let mut data = s.as_bytes().to_vec();
        data.push(0); // null terminator
        self.add_scalar_sample(&data);
    }

    /// Add an array sample.
    pub fn add_array_sample(&mut self, data: &[u8], dims: &[usize]) {
        if let OPropertyData::Array(samples) = &mut self.data {
            let sample_index = samples.len() as u32;
            let is_same = samples.last().map_or(false, |prev| {
                prev.data == data && prev.dims.as_slice() == dims
            });
            samples.push(ArraySampleWithDigest::new(data.to_vec(), dims.to_vec()));
            if self.is_scalar_like && dims.iter().product::<usize>() != 1 {
                self.is_scalar_like = false;
            }
            if sample_index == 0 {
                self.first_changed_index = 0;
                self.last_changed_index = 0;
            } else if !is_same {
                if self.first_changed_index == 0 {
                    self.first_changed_index = sample_index;
                }
                self.last_changed_index = sample_index;
            }
        }
    }

    /// Add an array sample with pre-computed digest (for raw copy).
    pub fn add_array_sample_with_digest(&mut self, data: &[u8], dims: &[usize], digest: SampleDigest) {
        if let OPropertyData::Array(samples) = &mut self.data {
            let sample_index = samples.len() as u32;
            let is_same = samples.last().map_or(false, |prev| {
                prev.data == data && prev.dims.as_slice() == dims
            });
            samples.push(ArraySampleWithDigest::with_digest(data.to_vec(), dims.to_vec(), digest));
            if self.is_scalar_like && dims.iter().product::<usize>() != 1 {
                self.is_scalar_like = false;
            }
            if sample_index == 0 {
                self.first_changed_index = 0;
                self.last_changed_index = 0;
            } else if !is_same {
                if self.first_changed_index == 0 {
                    self.first_changed_index = sample_index;
                }
                self.last_changed_index = sample_index;
            }
        }
    }

    /// Add array sample from Pod slice.
    pub fn add_array_pod<T: bytemuck::Pod>(&mut self, values: &[T]) {
        let data = bytemuck::cast_slice(values);
        self.add_array_sample(data, &[values.len()]);
    }

    /// Add a child property (for compound).
    pub fn add_child(&mut self, prop: OProperty) -> Option<&mut OProperty> {
        if let OPropertyData::Compound(children) = &mut self.data {
            children.push(prop);
            children.last_mut()
        } else {
            None
        }
    }

    /// Get or create an array child property by name.
    ///
    /// If a property with the given name exists, returns it.
    /// Otherwise creates a new array property and returns it.
    pub fn get_or_create_array_child(&mut self, name: &str, dt: DataType) -> &mut OProperty {
        if let OPropertyData::Compound(children) = &mut self.data {
            if let Some(idx) = children.iter().position(|p| p.name == name) {
                return &mut children[idx];
            }
            children.push(OProperty::array(name, dt));
            children.last_mut().unwrap()
        } else {
            panic!("get_or_create_array_child called on non-compound property")
        }
    }

    /// Get or create a scalar child property by name.
    ///
    /// If a property with the given name exists, returns it.
    /// Otherwise creates a new scalar property and returns it.
    pub fn get_or_create_scalar_child(&mut self, name: &str, dt: DataType) -> &mut OProperty {
        if let OPropertyData::Compound(children) = &mut self.data {
            if let Some(idx) = children.iter().position(|p| p.name == name) {
                return &mut children[idx];
            }
            children.push(OProperty::scalar(name, dt));
            children.last_mut().unwrap()
        } else {
            panic!("get_or_create_scalar_child called on non-compound property")
        }
    }

    /// Get or create a compound child property by name.
    ///
    /// If a property with the given name exists, returns it.
    /// Otherwise creates a new compound property and returns it.
    pub fn get_or_create_compound_child(&mut self, name: &str) -> &mut OProperty {
        if let OPropertyData::Compound(children) = &mut self.data {
            if let Some(idx) = children.iter().position(|p| p.name == name) {
                return &mut children[idx];
            }
            children.push(OProperty::compound(name));
            children.last_mut().unwrap()
        } else {
            panic!("get_or_create_compound_child called on non-compound property")
        }
    }

    /// Get number of samples.
    pub fn getNumSamples(&self) -> usize {
        match &self.data {
            OPropertyData::Scalar(s) => s.len(),
            OPropertyData::Array(s) => s.len(),
            OPropertyData::Compound(_) => 0,
        }
    }
}
