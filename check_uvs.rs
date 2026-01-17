use alembic::abc::IArchive;
use alembic::geom::IPolyMesh;

fn main() {
    let archive = IArchive::open("data/Abc/bmw.abc").unwrap();
    let root = archive.getTop();
    
    fn check_uvs(obj: &alembic::abc::IObject, depth: usize) {
        if let Some(mesh) = IPolyMesh::new(obj) {
            let name = obj.getName();
            let has_uvs = mesh.has_uvs();
            if has_uvs {
                println\!("MESH {} has UVs\!", name);
            }
        }
        for child in obj.getChildren() {
            check_uvs(&child, depth + 1);
        }
    }
    
    check_uvs(&root, 0);
    println\!("Done checking UVs");
}
