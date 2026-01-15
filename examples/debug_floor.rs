//! Debug floor.abc mesh data

use alembic::abc::IArchive;
use alembic::geom::{IPolyMesh, IXform};

fn main() {
    let archive = IArchive::open("data/Abc/floor.abc").expect("Failed to open floor.abc");
    let root = archive.getTop();
    
    visit(&root, 0);
}

fn visit(obj: &alembic::abc::IObject, depth: usize) {
    let indent = "  ".repeat(depth);
    let name = obj.getName();
    
    // Check if it's an xform
    if let Some(xform) = IXform::new(obj) {
        println!("{indent}[XFORM] {name}");
        if let Ok(sample) = xform.get_sample(0) {
            let m = sample.matrix();
            println!("{indent}  Matrix:");
            println!("{indent}    [{:10.4} {:10.4} {:10.4} {:10.4}]", m.x_axis.x, m.x_axis.y, m.x_axis.z, m.x_axis.w);
            println!("{indent}    [{:10.4} {:10.4} {:10.4} {:10.4}]", m.y_axis.x, m.y_axis.y, m.y_axis.z, m.y_axis.w);
            println!("{indent}    [{:10.4} {:10.4} {:10.4} {:10.4}]", m.z_axis.x, m.z_axis.y, m.z_axis.z, m.z_axis.w);
            println!("{indent}    [{:10.4} {:10.4} {:10.4} {:10.4}]", m.w_axis.x, m.w_axis.y, m.w_axis.z, m.w_axis.w);
        }
    }
    
    // Check if it's a polymesh
    if let Some(mesh) = IPolyMesh::new(obj) {
        println!("{indent}[MESH] {name}");
        
        if let Ok(sample) = mesh.get_sample(0) {
            println!("{indent}  Positions: {}", sample.positions.len());
            println!("{indent}  Face counts: {} = {:?}", sample.face_counts.len(), sample.face_counts);
            println!("{indent}  Face indices: {}", sample.face_indices.len());
            
            // Check index validity
            let max_idx = sample.face_indices.iter().max().copied().unwrap_or(0);
            let min_idx = sample.face_indices.iter().min().copied().unwrap_or(0);
            println!("{indent}  Index range: {} to {} (valid: 0-{})", min_idx, max_idx, sample.positions.len() - 1);
            
            // Print ALL positions
            println!("{indent}  ALL {} positions:", sample.positions.len());
            for (i, p) in sample.positions.iter().enumerate() {
                println!("{indent}    [{i:2}] ({:10.4}, {:10.4}, {:10.4})", p.x, p.y, p.z);
            }
            
            // Print all indices
            println!("{indent}  Face indices (all {}):", sample.face_indices.len());
            println!("{indent}    {:?}", sample.face_indices);
            
            // Simulate triangulation for first face
            println!("{indent}  \n  === Triangulation test (first 2 faces) ===");
            let mut idx_offset = 0usize;
            for face_idx in 0..2 {
                let count = sample.face_counts[face_idx] as usize;
                let face_vertex_indices: Vec<usize> = (0..count)
                    .map(|i| sample.face_indices[idx_offset + i] as usize)
                    .collect();
                println!("{indent}  Face {face_idx}: indices = {:?}", face_vertex_indices);
                
                // Fan triangulation
                for i in 1..count - 1 {
                    let i0 = face_vertex_indices[0];
                    let i1 = face_vertex_indices[i];
                    let i2 = face_vertex_indices[i + 1];
                    
                    let p0 = sample.positions[i0];
                    let p1 = sample.positions[i1];
                    let p2 = sample.positions[i2];
                    
                    println!("{indent}    Triangle: {i0}->{i1}->{i2}");
                    println!("{indent}      v{i0} = ({:.4}, {:.4}, {:.4})", p0.x, p0.y, p0.z);
                    println!("{indent}      v{i1} = ({:.4}, {:.4}, {:.4})", p1.x, p1.y, p1.z);
                    println!("{indent}      v{i2} = ({:.4}, {:.4}, {:.4})", p2.x, p2.y, p2.z);
                }
                
                idx_offset += count;
            }
        }
    } else {
        println!("{indent}[obj] {name}");
    }
    
    // Recurse
    for child in obj.getChildren() {
        visit(&child, depth + 1);
    }
}
