//! Smooth normals calculation with angle threshold

use glam::Vec3;
use std::collections::HashMap;

/// Data needed to recalculate smooth normals dynamically
#[derive(Clone)]
pub struct SmoothNormalData {
    /// Position hash -> list of (vertex_index, face_normal)
    pub position_groups: HashMap<u64, Vec<(usize, Vec3)>>,
    /// Original positions for each vertex
    pub positions: Vec<Vec3>,
    /// Face normal for each vertex
    pub face_normals: Vec<Vec3>,
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
            positions: positions.to_vec(),
            face_normals: face_normals.to_vec(),
        }
    }
    
    /// Recalculate smooth normals with given angle threshold (degrees)
    pub fn calculate(&self, angle_deg: f32) -> Vec<Vec3> {
        let cos_threshold = (angle_deg.to_radians()).cos();
        let mut normals = vec![Vec3::ZERO; self.positions.len()];
        
        for group in self.position_groups.values() {
            // For each vertex in the group, average normals from faces within angle threshold
            for &(idx, face_n) in group {
                let mut sum = Vec3::ZERO;
                let mut count = 0;
                
                for &(_, other_n) in group {
                    let dot = face_n.dot(other_n);
                    if dot >= cos_threshold {
                        sum += other_n;
                        count += 1;
                    }
                }
                
                if count > 0 {
                    normals[idx] = (sum / count as f32).normalize_or_zero();
                } else {
                    normals[idx] = face_n;
                }
            }
        }
        
        normals
    }
}

/// Hash position for grouping (quantized to avoid float precision issues)
fn pos_hash(p: Vec3) -> u64 {
    // Quantize to ~0.0001 precision
    let scale = 10000.0;
    let x = (p.x * scale).round() as i32;
    let y = (p.y * scale).round() as i32;
    let z = (p.z * scale).round() as i32;
    
    // Combine into hash
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    (x, y, z).hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_smooth_normals() {
        // Simple cube corner - 3 faces meeting
        let positions = vec![
            Vec3::ZERO, Vec3::ZERO, Vec3::ZERO,  // 3 verts at same position
        ];
        let face_normals = vec![
            Vec3::X,  // face pointing +X
            Vec3::Y,  // face pointing +Y
            Vec3::Z,  // face pointing +Z
        ];
        
        let data = SmoothNormalData::from_vertices(&positions, &face_normals);
        
        // With 180 degree threshold - all should average
        let smooth = data.calculate(180.0);
        let expected = (Vec3::X + Vec3::Y + Vec3::Z).normalize();
        assert!((smooth[0] - expected).length() < 0.001);
        
        // With 60 degree threshold - none should merge (90 degrees between)
        let smooth = data.calculate(60.0);
        assert!((smooth[0] - Vec3::X).length() < 0.001);
        assert!((smooth[1] - Vec3::Y).length() < 0.001);
        assert!((smooth[2] - Vec3::Z).length() < 0.001);
    }
}
