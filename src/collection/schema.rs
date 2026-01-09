//! Collection schema implementation.
//!
//! Provides reading of collection data from Alembic files.

use crate::abc::IObject;
use super::COLLECTIONS_SCHEMA;

/// A single collection containing object paths.
#[derive(Clone, Debug, Default)]
pub struct Collection {
    /// Collection name.
    pub name: String,
    /// Object paths in this collection.
    pub paths: Vec<String>,
}

impl Collection {
    /// Create an empty collection.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            paths: Vec::new(),
        }
    }
    
    /// Add a path to the collection.
    pub fn add(&mut self, path: &str) {
        self.paths.push(path.to_string());
    }
    
    /// Check if the collection contains a path.
    pub fn contains(&self, path: &str) -> bool {
        self.paths.iter().any(|p| p == path)
    }
    
    /// Get number of paths.
    pub fn len(&self) -> usize {
        self.paths.len()
    }
    
    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.paths.is_empty()
    }
    
    /// Iterate over paths.
    pub fn iter(&self) -> impl Iterator<Item = &str> {
        self.paths.iter().map(|s| s.as_str())
    }
}

/// Input collections schema reader.
/// 
/// Collections allow grouping objects by name without changing hierarchy.
/// Common uses include render passes, selection sets, and object groups.
pub struct ICollections<'a> {
    object: &'a IObject<'a>,
}

impl<'a> ICollections<'a> {
    /// Wrap an IObject as ICollections.
    /// Returns None if the object doesn't have the Collections schema.
    pub fn new(object: &'a IObject<'a>) -> Option<Self> {
        if object.matches_schema(COLLECTIONS_SCHEMA) {
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
    
    /// Get the number of collections.
    pub fn num_collections(&self) -> usize {
        let props = self.object.properties();
        let Some(coll_prop) = props.property_by_name(".collections") else {
            return 0;
        };
        let Some(coll) = coll_prop.as_compound() else {
            return 0;
        };
        coll.num_properties()
    }
    
    /// Get collection names.
    pub fn collection_names(&self) -> Vec<String> {
        let props = self.object.properties();
        let Some(coll_prop) = props.property_by_name(".collections") else {
            return Vec::new();
        };
        let Some(coll) = coll_prop.as_compound() else {
            return Vec::new();
        };
        coll.property_names()
    }
    
    /// Get a collection by name.
    pub fn get(&self, name: &str) -> Option<Collection> {
        let props = self.object.properties();
        let coll_prop = props.property_by_name(".collections")?;
        let coll = coll_prop.as_compound()?;
        let col_prop = coll.property_by_name(name)?;
        let array = col_prop.as_array()?;
        
        // Read the array of strings (paths)
        let data = array.read_sample_vec(0).ok()?;
        
        let mut collection = Collection::new(name);
        
        // Parse null-terminated strings from the data
        let mut start = 0;
        for (i, &byte) in data.iter().enumerate() {
            if byte == 0 {
                if i > start {
                    if let Ok(s) = String::from_utf8(data[start..i].to_vec()) {
                        collection.add(&s);
                    }
                }
                start = i + 1;
            }
        }
        
        // Handle case where there's no trailing null
        if start < data.len() {
            if let Ok(s) = String::from_utf8(data[start..].to_vec()) {
                if !s.is_empty() {
                    collection.add(&s);
                }
            }
        }
        
        Some(collection)
    }
    
    /// Get a collection by index.
    pub fn collection(&self, index: usize) -> Option<Collection> {
        let names = self.collection_names();
        names.get(index).and_then(|name| self.get(name))
    }
    
    /// Check if a collection exists.
    pub fn has_collection(&self, name: &str) -> bool {
        let props = self.object.properties();
        let Some(coll_prop) = props.property_by_name(".collections") else {
            return false;
        };
        let Some(coll) = coll_prop.as_compound() else {
            return false;
        };
        coll.has_property(name)
    }
    
    /// Get all collections.
    pub fn all(&self) -> Vec<Collection> {
        self.collection_names()
            .iter()
            .filter_map(|name| self.get(name))
            .collect()
    }
    
    /// Check if this schema is valid.
    #[inline]
    pub fn valid(&self) -> bool {
        true
    }
}

// ============================================================================
// Collection Utilities
// ============================================================================

/// Check if an object path exists in the archive.
/// 
/// Traverses the hierarchy to verify the path is valid.
pub fn path_exists(root: &IObject, path: &str) -> bool {
    let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    
    if parts.is_empty() {
        return false;
    }
    
    // Check first level
    let Some(first) = root.child_by_name(parts[0]) else {
        return false;
    };
    
    // For single-level paths, we're done
    if parts.len() == 1 {
        return true;
    }
    
    // For deeper paths, continue checking
    check_path_recursive(&first, &parts[1..])
}

/// Recursive helper to check path existence.
fn check_path_recursive(current: &IObject, remaining: &[&str]) -> bool {
    if remaining.is_empty() {
        return true;
    }
    
    let Some(child) = current.child_by_name(remaining[0]) else {
        return false;
    };
    
    if remaining.len() == 1 {
        return true;
    }
    
    check_path_recursive(&child, &remaining[1..])
}

/// Filter collection paths to only those that exist in the archive.
/// 
/// Returns paths from the collection that can be resolved in the archive.
pub fn resolve_collection_paths(root: &IObject, collection: &Collection) -> Vec<String> {
    collection.paths
        .iter()
        .filter(|path| path_exists(root, path))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_collection_basic() {
        let mut coll = Collection::new("render_objects");
        assert!(coll.is_empty());
        
        coll.add("/root/mesh1");
        coll.add("/root/mesh2");
        
        assert_eq!(coll.len(), 2);
        assert!(!coll.is_empty());
        assert!(coll.contains("/root/mesh1"));
        assert!(!coll.contains("/root/mesh3"));
    }
    
    #[test]
    fn test_collection_iter() {
        let mut coll = Collection::new("test");
        coll.add("/a");
        coll.add("/b");
        coll.add("/c");
        
        let paths: Vec<&str> = coll.iter().collect();
        assert_eq!(paths, vec!["/a", "/b", "/c"]);
    }
}
