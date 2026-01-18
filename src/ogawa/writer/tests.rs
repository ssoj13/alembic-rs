use super::*;
use std::fs::File;
use std::io::Read;
use tempfile::NamedTempFile;

#[test]
fn test_write_empty_archive() -> crate::util::Result<()> {
    let temp = NamedTempFile::new()?;
    let path = temp.path();

    let archive = OArchive::create(path)?;
    archive.close()?;

    let mut file = File::open(path)?;
    let mut header = [0u8; crate::ogawa::format::HEADER_SIZE];
    file.read_exact(&mut header)?;

    assert_eq!(&header[0..5], crate::ogawa::format::OGAWA_MAGIC);
    assert_eq!(header[crate::ogawa::format::FROZEN_OFFSET], crate::ogawa::format::FROZEN_FLAG);
    assert_eq!(header[crate::ogawa::format::VERSION_OFFSET], 0);
    assert_eq!(header[crate::ogawa::format::VERSION_OFFSET + 1], 1);

    Ok(())
}

#[test]
fn test_write_and_read_archive() -> crate::util::Result<()> {
    let temp = NamedTempFile::new()?;
    let path = temp.path();

    let mut archive = OArchive::create(path)?;

    let mut root = OObject::new("");
    let child = OObject::new("test_child");
    root.add_child(child);

    archive.write_archive(&root)?;

    let reader = super::super::IArchive::open(path)?;
    assert!(reader.is_valid());
    assert!(reader.is_frozen());

    Ok(())
}

#[test]
fn test_write_polymesh() -> crate::util::Result<()> {
    let temp = NamedTempFile::new()?;
    let path = temp.path();

    let mut archive = OArchive::create(path)?;

    let mut mesh = OPolyMesh::new("triangle");
    mesh.add_sample(&OPolyMeshSample::new(
        vec![
            glam::Vec3::new(0.0, 0.0, 0.0),
            glam::Vec3::new(1.0, 0.0, 0.0),
            glam::Vec3::new(0.5, 1.0, 0.0),
        ],
        vec![3],
        vec![0, 1, 2],
    ));

    let mut root = OObject::new("");
    root.add_child(mesh.build());

    archive.write_archive(&root)?;

    let reader = super::super::IArchive::open(path)?;
    assert!(reader.is_valid());

    Ok(())
}

#[test]
fn test_write_xform() -> crate::util::Result<()> {
    let temp = NamedTempFile::new()?;
    let path = temp.path();

    let mut archive = OArchive::create(path)?;

    let mut xform = OXform::new("transform");
    xform.add_sample(OXformSample::identity());
    xform.add_sample(OXformSample::from_matrix(
        glam::Mat4::from_translation(glam::Vec3::new(1.0, 2.0, 3.0)),
        true,
    ));

    let mut root = OObject::new("");
    root.add_child(xform.build());

    archive.write_archive(&root)?;

    let reader = super::super::IArchive::open(path)?;
    assert!(reader.is_valid());

    Ok(())
}
