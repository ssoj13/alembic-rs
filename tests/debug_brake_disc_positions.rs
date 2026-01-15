//! Debug test to compare brake_disc vertex positions across all wheels
use alembic::abc::IArchive;
use alembic::geom::IPolyMesh;

fn find_brake_disc_meshes(obj: &alembic::abc::IObject, path: &str) -> Vec<(String, Vec<[f64; 3]>)> {
    let mut results = Vec::new();
    let name = obj.name();
    let current_path = if path.is_empty() {
        name.to_string()
    } else {
        format!("{}/{}", path, name)
    };
    
    // Check if this is a brake_disc mesh
    if name == "brake_disc" || name == "brake_discShape" {
        if let Some(mesh) = IPolyMesh::new(obj) {
            if let Ok(sample) = mesh.get_sample(0) {
                let positions = &sample.positions;
                // Get first 5 vertices
                let verts: Vec<[f64; 3]> = positions.iter().take(5).map(|p| {
                    [p[0] as f64, p[1] as f64, p[2] as f64]
                }).collect();
                results.push((current_path.clone(), verts));
            }
        }
    }
    
    // Recurse children
    for i in 0..obj.num_children() {
        if let Some(child) = obj.child(i) {
            results.extend(find_brake_disc_meshes(&child, &current_path));
        }
    }
    
    results
}

#[test]
fn debug_brake_disc_positions() {
    let archive = IArchive::open("data/bmw.abc").expect("Failed to open archive");
    let root = archive.root();
    
    let brake_discs = find_brake_disc_meshes(&root, "");
    
    println!("\n=== Brake Disc Vertex Positions ===\n");
    
    for (path, verts) in &brake_discs {
        println!("{}:", path);
        for (i, v) in verts.iter().enumerate() {
            println!("  v[{}]: ({:.4}, {:.4}, {:.4})", i, v[0], v[1], v[2]);
        }
        println!();
    }
    
    // Check if all brake_discs have different positions (as they should in world space)
    if brake_discs.len() >= 2 {
        let first = &brake_discs[0].1;
        let mut all_same = true;
        for (path, verts) in brake_discs.iter().skip(1) {
            if verts != first {
                all_same = false;
            }
        }
        
        if all_same {
            println!("WARNING: All brake_disc meshes have IDENTICAL vertex positions!");
            println!("This might indicate a data sharing issue.");
        } else {
            println!("OK: Brake disc meshes have DIFFERENT vertex positions (world-space baked)");
        }
    }
}
