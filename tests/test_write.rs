use alembic::ogawa::writer::{OArchive, OObject, OPolyMesh, OPolyMeshSample};
use glam::Vec3;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a simple triangle
    let positions = vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(0.5, 1.0, 0.0),
    ];
    let face_counts = vec![3i32];
    let face_indices = vec![0i32, 1, 2];

    // Write archive
    let mut archive = OArchive::create("test_triangle.abc")?;
    
    let mut mesh = OPolyMesh::new("triangle");
    mesh.add_sample(&OPolyMeshSample::new(
        positions,
        face_counts,
        face_indices,
    ));

    let mut root = OObject::new("");
    root.add_child(mesh.build());

    archive.write_archive(&root)?;
    archive.close()?;
    
    println!("Successfully created test_triangle.abc");
    Ok(())
}