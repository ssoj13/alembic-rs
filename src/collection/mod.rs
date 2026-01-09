//! AbcCollection module - Collection schemas for Alembic.
//!
//! Collections in Alembic provide a way to group objects together by name
//! without modifying the scene hierarchy. They are useful for render passes,
//! selection sets, and organizational grouping.
//!
//! ## Key Concepts
//!
//! - **Collection**: Named group of object paths
//! - **ICollections**: Schema containing multiple named collections
//!
//! ## Example
//!
//! ```ignore
//! use alembic::collection::ICollections;
//!
//! if let Some(collections) = ICollections::new(&object) {
//!     for name in collections.collection_names() {
//!         println!("Collection: {}", name);
//!         for path in collections.get(&name) {
//!             println!("  - {}", path);
//!         }
//!     }
//! }
//! ```

mod schema;

pub use schema::*;

/// Collections schema identifier.
pub const COLLECTIONS_SCHEMA: &str = "AbcCollection_Collection_v1";
