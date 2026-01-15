//! Smooth normals calculation with angle threshold

use glam::Vec3;
use std::collections::HashMap;

/// Data needed to recalculate smooth normals dynamically
#[derive(Clone)]
pub struct SmoothNormalData {
    /// Position hash -> list of (vertex_index, face_normal)
    position_groups: HashMap<u64, Vec<(usize, Vec3)>>,
    /// Number of vertices
    vertex_count: usize,
}

impl SmoothNormalData {
    /// Build smooth normal data from vertices
    pub fn from_vertices(positions: &[Vec3], face_normals: &[Vec3]) -> Self {
        let mut position_groups: HashMap<u64, Vec<(usize, Vec3)>> = HashMap::new();
        
        for (idx, (pos, fn_)) in positions.iter().zip(face_normals.iter()).enumerate() {
            let hash = pos_hash(*pos);
            position_groups.entry(hash).or_default().push((idx, *fn_));
        }
        
        Self {
            position_groups,
            vertex_count: positions.len(),
        }
    }
    
    /// Recalculate smooth normals with given angle threshold (degrees)
    pub fn calculate(&self, angle_deg: f32) -> Vec<Vec3> {
        let cos_threshold = (angle_deg.to_radians()).cos();
        let mut normals = vec![Vec3::ZERO; self.vertex_count];
        
        for group in self.position_groups.values() {
            for &(idx, face_n) in group {
                let mut sum = Vec3::ZERO;
                let mut count = 0;
                
                for &(_, other_n) in group {
                    if face_n.dot(other_n) >= cos_threshold {
                        sum += other_n;
                        count += 1;
                    }
                }
                
                normals[idx] = if count > 0 {
                    (sum / count as f32).normalize_or_zero()
                } else {
                    face_n
                };
            }
        }
        
        normals
    }
}

/// Hash position for grouping
fn pos_hash(p: Vec3) -> u64 {
    let scale = 10000.0;
    let x = (p.x * scale).round() as i32;
    let y = (p.y * scale).round() as i32;
    let z = (p.z * scale).round() as i32;
    
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    (x, y, z).hash(&mut hasher);
    hasher.finish()
}
