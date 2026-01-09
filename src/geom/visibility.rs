//! Visibility support for Alembic geometry.
//!
//! Objects in Alembic can have visibility properties that control whether
//! they should be rendered or displayed. Visibility can be:
//! - Deferred: inherit from parent
//! - Hidden: explicitly hidden
//! - Visible: explicitly visible

use crate::abc::IObject;
use crate::core::SampleSelector;

/// Visibility property name constant.
pub const VISIBILITY_PROPERTY_NAME: &str = "visible";

/// Object visibility state.
///
/// Controls whether an object should be visible in the scene.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i8)]
pub enum ObjectVisibility {
    /// Visibility is deferred to parent.
    /// Walk up the hierarchy to find an explicit visibility value.
    /// If root is reached, object is visible.
    #[default]
    Deferred = -1,
    
    /// Object is explicitly hidden.
    Hidden = 0,
    
    /// Object is explicitly visible.
    Visible = 1,
}

impl ObjectVisibility {
    /// Parse from i8 value (as stored in property).
    pub fn from_i8(value: i8) -> Self {
        match value {
            0 => Self::Hidden,
            1 => Self::Visible,
            _ => Self::Deferred, // -1 or any other value
        }
    }
    
    /// Convert to i8 for storage.
    pub fn to_i8(self) -> i8 {
        match self {
            Self::Deferred => -1,
            Self::Hidden => 0,
            Self::Visible => 1,
        }
    }
    
    /// Check if this is deferred visibility.
    pub fn is_deferred(self) -> bool {
        matches!(self, Self::Deferred)
    }
    
    /// Check if this is explicitly hidden.
    pub fn is_hidden(self) -> bool {
        matches!(self, Self::Hidden)
    }
    
    /// Check if this is explicitly visible.
    pub fn is_visible(self) -> bool {
        matches!(self, Self::Visible)
    }
}

impl From<i8> for ObjectVisibility {
    fn from(value: i8) -> Self {
        Self::from_i8(value)
    }
}

impl From<ObjectVisibility> for i8 {
    fn from(vis: ObjectVisibility) -> Self {
        vis.to_i8()
    }
}

/// Get visibility property from an object.
///
/// Returns None if the object doesn't have a visibility property.
pub fn get_visibility_property(obj: &IObject<'_>) -> Option<ObjectVisibility> {
    let props = obj.properties();
    
    // Look for "visible" property
    if let Some(prop) = props.property_by_name(VISIBILITY_PROPERTY_NAME) {
        if prop.is_scalar() {
            if let Some(scalar) = prop.as_scalar() {
                // Read as i8 (char in C++)
                let mut buf = [0u8; 1];
                if scalar.read_sample(0, &mut buf).is_ok() {
                    return Some(ObjectVisibility::from_i8(buf[0] as i8));
                }
            }
        }
    }
    
    None
}

/// Get visibility of an object at a specific sample.
///
/// If the object doesn't have a visibility property, returns Deferred.
pub fn get_visibility(obj: &IObject<'_>, sel: impl Into<SampleSelector>) -> ObjectVisibility {
    let props = obj.properties();
    let sel = sel.into();
    
    if let Some(prop) = props.property_by_name(VISIBILITY_PROPERTY_NAME) {
        if prop.is_scalar() {
            if let Some(scalar) = prop.as_scalar() {
                let mut buf = [0u8; 1];
                let index = match sel {
                    SampleSelector::Index(i) => i,
                    _ => 0,
                };
                if scalar.read_sample(index, &mut buf).is_ok() {
                    return ObjectVisibility::from_i8(buf[0] as i8);
                }
            }
        }
    }
    
    ObjectVisibility::Deferred
}

/// Check if an object or any of its ancestors is hidden.
///
/// Walks up the hierarchy looking for explicit visibility.
/// Returns true if the object should be visible, false if hidden.
///
/// Note: This function requires traversing the hierarchy which may be expensive.
/// For now, it only checks the immediate object since we don't have parent access.
pub fn is_visible(obj: &IObject<'_>, sel: impl Into<SampleSelector>) -> bool {
    let vis = get_visibility(obj, sel);
    
    match vis {
        ObjectVisibility::Visible => true,
        ObjectVisibility::Hidden => false,
        ObjectVisibility::Deferred => {
            // Would need to walk up hierarchy, but we don't have parent access yet
            // Default to visible when deferred
            true
        }
    }
}

/// Check if any ancestor of the object is explicitly invisible.
///
/// Returns true if any ancestor has kVisibilityHidden.
/// 
/// NOTE: This version only checks the immediate object due to Rust ownership.
/// Use `is_ancestor_invisible_in_archive()` for full hierarchy traversal.
pub fn is_ancestor_invisible(obj: &IObject<'_>, sel: impl Into<SampleSelector>) -> bool {
    // Without parent access, we can only check the object itself
    get_visibility(obj, sel) == ObjectVisibility::Hidden
}

/// Check if any ancestor in the path is explicitly invisible.
/// 
/// Traverses from root to the object, checking visibility at each level.
/// Returns true if any ancestor (not including the object itself) is hidden.
/// 
/// # Arguments
/// * `archive` - The archive to search in
/// * `obj_path` - Full path of the object (e.g., "/root/parent/child")
/// * `sel` - Sample selector for animated visibility
pub fn is_ancestor_invisible_in_archive(
    archive: &crate::abc::IArchive,
    obj_path: &str,
    sel: impl Into<SampleSelector> + Copy,
) -> bool {
    // Parse path into segments
    let parts: Vec<&str> = obj_path.split('/').filter(|s| !s.is_empty()).collect();
    if parts.len() <= 1 {
        return false; // No ancestors (root or single-level)
    }
    
    // Check visibility at each ancestor level
    check_ancestors_recursive(&archive.root(), &parts[..parts.len()-1], sel)
}

/// Recursively check ancestors for hidden visibility.
fn check_ancestors_recursive(
    current: &IObject<'_>,
    remaining: &[&str],
    sel: impl Into<SampleSelector> + Copy,
) -> bool {
    if remaining.is_empty() {
        return false;
    }
    
    // Get first child in path
    let Some(child) = current.child_by_name(remaining[0]) else {
        return false; // Path not found
    };
    
    // Check if this ancestor is hidden
    if get_visibility(&child, sel) == ObjectVisibility::Hidden {
        return true;
    }
    
    // Check remaining ancestors
    if remaining.len() > 1 {
        check_ancestors_recursive(&child, &remaining[1..], sel)
    } else {
        false
    }
}

// ============================================================================
// Output (Write) Support
// ============================================================================

use crate::ogawa::writer::OProperty;
use crate::util::DataType;

/// Create a visibility property for writing.
///
/// Returns an OProperty configured for visibility values.
pub fn create_visibility_property() -> OProperty {
    OProperty::scalar(VISIBILITY_PROPERTY_NAME, DataType::INT8)
}

/// Add visibility sample to a property.
///
/// The property should have been created with `create_visibility_property()`.
pub fn add_visibility_sample(prop: &mut OProperty, vis: ObjectVisibility) {
    prop.add_scalar_pod(&vis.to_i8());
}

/// OVisibilityProperty - convenience wrapper for writing visibility.
pub struct OVisibilityProperty {
    property: OProperty,
}

impl OVisibilityProperty {
    /// Create a new visibility property.
    pub fn new() -> Self {
        Self {
            property: create_visibility_property(),
        }
    }
    
    /// Add a visibility sample.
    pub fn set(&mut self, vis: ObjectVisibility) {
        add_visibility_sample(&mut self.property, vis);
    }
    
    /// Add a "visible" sample.
    pub fn set_visible(&mut self) {
        self.set(ObjectVisibility::Visible);
    }
    
    /// Add a "hidden" sample.
    pub fn set_hidden(&mut self) {
        self.set(ObjectVisibility::Hidden);
    }
    
    /// Add a "deferred" sample (inherit from parent).
    pub fn set_deferred(&mut self) {
        self.set(ObjectVisibility::Deferred);
    }
    
    /// Get the underlying property for adding to an object.
    pub fn into_property(self) -> OProperty {
        self.property
    }
    
    /// Get mutable reference to underlying property.
    pub fn property_mut(&mut self) -> &mut OProperty {
        &mut self.property
    }
}

impl Default for OVisibilityProperty {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_visibility_conversion() {
        assert_eq!(ObjectVisibility::from_i8(-1), ObjectVisibility::Deferred);
        assert_eq!(ObjectVisibility::from_i8(0), ObjectVisibility::Hidden);
        assert_eq!(ObjectVisibility::from_i8(1), ObjectVisibility::Visible);
        assert_eq!(ObjectVisibility::from_i8(42), ObjectVisibility::Deferred);
        
        assert_eq!(ObjectVisibility::Deferred.to_i8(), -1);
        assert_eq!(ObjectVisibility::Hidden.to_i8(), 0);
        assert_eq!(ObjectVisibility::Visible.to_i8(), 1);
    }
    
    #[test]
    fn test_visibility_checks() {
        assert!(ObjectVisibility::Deferred.is_deferred());
        assert!(!ObjectVisibility::Deferred.is_hidden());
        assert!(!ObjectVisibility::Deferred.is_visible());
        
        assert!(!ObjectVisibility::Hidden.is_deferred());
        assert!(ObjectVisibility::Hidden.is_hidden());
        assert!(!ObjectVisibility::Hidden.is_visible());
        
        assert!(!ObjectVisibility::Visible.is_deferred());
        assert!(!ObjectVisibility::Visible.is_hidden());
        assert!(ObjectVisibility::Visible.is_visible());
    }
    
    #[test]
    fn test_ovisibility_property() {
        let mut vis = OVisibilityProperty::new();
        vis.set_visible();
        
        let prop = vis.into_property();
        assert_eq!(prop.name, "visible");
        assert_eq!(prop.num_samples(), 1);
    }
    
    #[test]
    fn test_visibility_samples() {
        let mut vis = OVisibilityProperty::new();
        vis.set_visible();
        vis.set_hidden();
        vis.set_deferred();
        
        let prop = vis.into_property();
        assert_eq!(prop.num_samples(), 3);
    }
}
