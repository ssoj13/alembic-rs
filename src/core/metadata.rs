//! Metadata for Alembic objects and properties.
//!
//! Metadata is stored as key-value pairs of strings and is used to
//! describe schemas, interpretations, and other attributes.

use smallvec::SmallVec;
use std::collections::HashMap;
use std::fmt;

/// Metadata storage - key-value pairs of strings.
///
/// Uses SmallVec optimization for common case of few entries.
#[derive(Clone, Default)]
pub struct MetaData {
    entries: SmallVec<[(String, String); 4]>,
}

impl MetaData {
    /// Create empty metadata.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set a metadata value.
    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        let key = key.into();
        let value = value.into();

        // Update existing or add new
        for (k, v) in &mut self.entries {
            if k == &key {
                *v = value;
                return;
            }
        }
        self.entries.push((key, value));
    }

    /// Get a metadata value by key.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.entries
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }

    /// Check if a key exists.
    pub fn contains(&self, key: &str) -> bool {
        self.entries.iter().any(|(k, _)| k == key)
    }

    /// Remove a key and return its value.
    pub fn remove(&mut self, key: &str) -> Option<String> {
        if let Some(pos) = self.entries.iter().position(|(k, _)| k == key) {
            Some(self.entries.remove(pos).1)
        } else {
            None
        }
    }

    /// Get the number of entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Clear all entries.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Iterate over key-value pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.entries.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }

    /// Serialize to Alembic metadata string format.
    /// Format: "key=value;key2=value2;..."
    pub fn serialize(&self) -> String {
        let mut result = String::new();
        for (i, (k, v)) in self.entries.iter().enumerate() {
            if i > 0 {
                result.push(';');
            }
            // Escape special characters
            result.push_str(&escape_metadata_string(k));
            result.push('=');
            result.push_str(&escape_metadata_string(v));
        }
        result
    }

    /// Parse from Alembic metadata string format.
    pub fn parse(s: &str) -> Self {
        let mut meta = Self::new();

        if s.is_empty() {
            return meta;
        }

        for part in split_metadata(s) {
            if let Some(eq_pos) = find_unescaped(part, b'=') {
                let key = unescape_metadata_string(&part[..eq_pos]);
                let value = unescape_metadata_string(&part[eq_pos + 1..]);
                if !key.is_empty() {
                    meta.set(key, value);
                }
            }
        }

        meta
    }

    // === Common metadata keys ===

    /// Schema title key.
    pub const SCHEMA_KEY: &'static str = "schema";

    /// Schema base type key.
    pub const SCHEMA_BASE_KEY: &'static str = "schemaBaseType";

    /// Interpretation key (e.g., "point", "vector", "normal").
    pub const INTERPRETATION_KEY: &'static str = "interpretation";

    /// Get schema title.
    pub fn schema(&self) -> Option<&str> {
        self.get(Self::SCHEMA_KEY)
    }

    /// Set schema title.
    pub fn set_schema(&mut self, schema: &str) {
        self.set(Self::SCHEMA_KEY, schema);
    }

    /// Get schema base type.
    pub fn schema_base(&self) -> Option<&str> {
        self.get(Self::SCHEMA_BASE_KEY)
    }

    /// Get interpretation.
    pub fn interpretation(&self) -> Option<&str> {
        self.get(Self::INTERPRETATION_KEY)
    }

    /// Check if this matches a schema title.
    pub fn matches_schema(&self, title: &str) -> bool {
        self.schema().map(|s| s == title).unwrap_or(false)
    }
}

impl fmt::Debug for MetaData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map()
            .entries(self.entries.iter().map(|(k, v)| (k, v)))
            .finish()
    }
}

impl From<HashMap<String, String>> for MetaData {
    fn from(map: HashMap<String, String>) -> Self {
        let mut meta = Self::new();
        for (k, v) in map {
            meta.set(k, v);
        }
        meta
    }
}

impl FromIterator<(String, String)> for MetaData {
    fn from_iter<T: IntoIterator<Item = (String, String)>>(iter: T) -> Self {
        let mut meta = Self::new();
        for (k, v) in iter {
            meta.set(k, v);
        }
        meta
    }
}

/// Escape special characters in metadata strings.
fn escape_metadata_string(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => result.push_str("\\\\"),
            ';' => result.push_str("\\;"),
            '=' => result.push_str("\\="),
            _ => result.push(c),
        }
    }
    result
}

/// Unescape special characters in metadata strings.
fn unescape_metadata_string(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(&next) = chars.peek() {
                match next {
                    '\\' | ';' | '=' => {
                        result.push(next);
                        chars.next();
                    }
                    _ => result.push(c),
                }
            } else {
                result.push(c);
            }
        } else {
            result.push(c);
        }
    }
    result
}

/// Find first unescaped occurrence of a character.
fn find_unescaped(s: &str, ch: u8) -> Option<usize> {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == ch {
            // Count preceding backslashes
            let mut backslashes = 0;
            let mut j = i;
            while j > 0 && bytes[j - 1] == b'\\' {
                backslashes += 1;
                j -= 1;
            }
            if backslashes % 2 == 0 {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

/// Split metadata string by semicolons, respecting escapes.
fn split_metadata(s: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0;
    let mut i = 0;
    let bytes = s.as_bytes();

    while i < bytes.len() {
        if bytes[i] == b';' {
            // Check if escaped
            let mut backslashes = 0;
            let mut j = i;
            while j > 0 && bytes[j - 1] == b'\\' {
                backslashes += 1;
                j -= 1;
            }
            if backslashes % 2 == 0 {
                // Not escaped
                parts.push(&s[start..i]);
                start = i + 1;
            }
        }
        i += 1;
    }

    if start < s.len() {
        parts.push(&s[start..]);
    }

    parts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata_basic() {
        let mut meta = MetaData::new();
        meta.set("key1", "value1");
        meta.set("key2", "value2");

        assert_eq!(meta.get("key1"), Some("value1"));
        assert_eq!(meta.get("key2"), Some("value2"));
        assert_eq!(meta.get("key3"), None);
        assert_eq!(meta.len(), 2);
    }

    #[test]
    fn test_metadata_update() {
        let mut meta = MetaData::new();
        meta.set("key", "value1");
        meta.set("key", "value2");

        assert_eq!(meta.get("key"), Some("value2"));
        assert_eq!(meta.len(), 1);
    }

    #[test]
    fn test_metadata_serialize() {
        let mut meta = MetaData::new();
        meta.set("schema", "AbcGeom_PolyMesh_v1");
        meta.set("interpretation", "point");

        let s = meta.serialize();
        assert!(s.contains("schema=AbcGeom_PolyMesh_v1"));
        assert!(s.contains("interpretation=point"));
    }

    #[test]
    fn test_metadata_parse() {
        let meta = MetaData::parse("schema=AbcGeom_PolyMesh_v1;interpretation=point");

        assert_eq!(meta.schema(), Some("AbcGeom_PolyMesh_v1"));
        assert_eq!(meta.interpretation(), Some("point"));
    }

    #[test]
    fn test_metadata_escape() {
        let mut meta = MetaData::new();
        meta.set("key=with;special", "value=with;special");

        let s = meta.serialize();
        let parsed = MetaData::parse(&s);

        assert_eq!(parsed.get("key=with;special"), Some("value=with;special"));
    }
}
