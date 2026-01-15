//! Debug test to verify all brake_disc meshes are collected by viewer
use alembic::abc::IArchive;
use alembic::viewer::mesh_converter::{collect_scene_cached, new_mesh_cache};

#[test]
fn debug_viewer_collect_brake_disc() {
    let archive = IArchive::open("data/bmw.abc").expect("Failed to open archive");
    let cache = new_mesh_cache();
    
    // Collect scene
    let scene = collect_scene_cached(&archive, 0, Some(&cache));
    
    println!("\n=== Collected Scene ===");
    println!("Total meshes: {}", scene.meshes.len());
    
    // Find all brake_disc meshes
    let brake_discs: Vec<_> = scene.meshes.iter()
        .filter(|m| m.name.contains("brake_disc") || m.path.contains("brake_disc"))
        .collect();
    
    println!("\nBrake disc meshes found: {}", brake_discs.len());
    
    for m in &brake_discs {
        println!("  {} -> path: {}", m.name, m.path);
        println!("    vertices: {}, indices: {}", m.vertices.len(), m.indices.len());
        println!("    bounds: min={:?} max={:?}", m.bounds.min, m.bounds.max);
        // Show first vertex position
        if let Some(v) = m.vertices.first() {
            println!("    first vertex: {:?}", v.position);
        }
    }
    
    // Check uniqueness
    if brake_discs.len() >= 2 {
        let first_pos = brake_discs[0].vertices.first().map(|v| v.position);
        let mut duplicates = 0;
        for bd in brake_discs.iter().skip(1) {
            let pos = bd.vertices.first().map(|v| v.position);
            if pos == first_pos {
                duplicates += 1;
                println!("\n  WARNING: {} has SAME first vertex as {}", bd.path, brake_discs[0].path);
            }
        }
        if duplicates > 0 {
            println!("\n  PROBLEM: {} meshes have duplicate vertex data!", duplicates);
        } else {
            println!("\n  OK: All brake_disc meshes have unique vertex data");
        }
    }
    
    assert_eq!(brake_discs.len(), 4, "Expected 4 brake_disc meshes");
}
