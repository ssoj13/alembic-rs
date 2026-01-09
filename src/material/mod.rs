//! AbcMaterial module - Material schemas for Alembic.
//!
//! This module provides material and shader network support for Alembic files.
//! Materials in Alembic are schema-based objects that describe shading networks
//! and material assignments.
//!
//! ## Key Concepts
//!
//! - **Material Schema**: Container for shader definitions and their parameters
//! - **Shader Network**: Graph of interconnected shader nodes
//! - **Material Assignment**: Binding materials to geometry via properties
//!
//! ## Example
//!
//! ```ignore
//! use alembic::material::IMaterial;
//!
//! // Check if an object has material assignments
//! if let Some(material) = IMaterial::new(&object) {
//!     for target in material.target_names() {
//!         println!("Target: {}", target);
//!         for shader_type in material.shader_type_names(&target) {
//!             println!("  Shader type: {}", shader_type);
//!         }
//!     }
//! }
//! ```

mod schema;

pub use schema::*;

/// Material schema identifier.
pub const MATERIAL_SCHEMA: &str = "AbcMaterial_Material_v1";

/// Property name for material assignments on geometry.
pub const MATERIAL_ASSIGN_PROP: &str = ".material.assign";

/// Property name for material bind paths.
pub const MATERIAL_BIND_PROP: &str = ".material.bind";
