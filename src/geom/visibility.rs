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
/// Note: Currently limited since we don't have full parent traversal.
pub fn is_ancestor_invisible(_obj: &IObject<'_>, _sel: impl Into<SampleSelector>) -> bool {
    // TODO: Implement when parent access is available
    // For now, return false (assume no invisible ancestors)
    false
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
}
