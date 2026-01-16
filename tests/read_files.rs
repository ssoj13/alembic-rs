//! Integration tests for reading real Alembic files.

use alembic::abc::IArchive;
use alembic::ogawa::IArchive as OgawaIArchive;
use alembic::geom::{IXform, XFORM_SCHEMA, IPolyMesh, POLYMESH_SCHEMA};

const CHESS3_PATH: &str = "data/Abc/chess3.abc";
const CHESS4_PATH: &str = "data/Abc/chess4.abc";
const BMW_PATH: &str = "data/Abc/bmw.abc";

#[test]
fn test_open_chess3() {
    let archive = OgawaIArchive::open(CHESS3_PATH).expect("Failed to open chess3.abc");
    
    println!("Version: {}", archive.version());
    println!("Frozen: {}", archive.is_frozen());
    
    let root = archive.root();
    println!("Root children: {}", root.num_children());
    
    // Explore first level
    for i in 0..root.num_children().min(10) {
        if root.is_child_group(i).unwrap_or(false) {
            println!("  Child {}: Group", i);
        } else {
            println!("  Child {}: Data", i);
        }
    }
    
    assert!(archive.is_valid());
}

#[test]
fn test_open_chess4() {
    let archive = OgawaIArchive::open(CHESS4_PATH).expect("Failed to open chess4.abc");
    assert!(archive.is_valid());
    println!("chess4.abc - Version: {}, Children: {}", 
             archive.version(), 
             archive.root().num_children());
}

#[test]
fn test_open_bmw() {
    let archive = OgawaIArchive::open(BMW_PATH).expect("Failed to open bmw.abc");
    assert!(archive.is_valid());
    println!("bmw.abc - Version: {}, Children: {}", 
             archive.version(), 
             archive.root().num_children());
}

#[test]
fn test_bmw_geometry() {
    let archive = IArchive::open(BMW_PATH).expect("Failed to open bmw.abc");
    let root = archive.getTop();
    
    println!("\n=== BMW Geometry Test ===");
    println!("Root children: {}", root.getNumChildren());
    
    let mut total_meshes = 0;
    let mut total_xforms = 0;
    let mut total_vertices = 0;
    let mut total_faces = 0;
    
    // Recursive scan
    fn scan_object(
        obj: &alembic::abc::IObject, 
        depth: usize,
        meshes: &mut usize,
        xforms: &mut usize,
        verts: &mut usize,
        faces: &mut usize,
    ) {
        let indent = "  ".repeat(depth);
        
        // Check for xform
        if let Some(xform) = IXform::new(obj) {
            *xforms += 1;
            if depth < 2 {
                println!("{}Xform: '{}'", indent, xform.getName());
            }
        }
        
        // Check for polymesh
        if let Some(mesh) = IPolyMesh::new(obj) {
            *meshes += 1;
            if let Ok(sample) = mesh.getSample(0) {
                *verts += sample.num_vertices();
                *faces += sample.num_faces();
                if depth < 3 {
                    println!("{}PolyMesh: '{}' - {} verts, {} faces", 
                        indent, mesh.getName(), sample.num_vertices(), sample.num_faces());
                }
            }
        }
        
        // Recurse children
        for child in obj.getChildren() {
            scan_object(&child, depth + 1, meshes, xforms, verts, faces);
        }
    }
    
    for child in root.getChildren() {
        scan_object(&child, 0, &mut total_meshes, &mut total_xforms, &mut total_vertices, &mut total_faces);
    }
    
    println!("\n--- BMW Summary ---");
    println!("Total Xforms: {}", total_xforms);
    println!("Total Meshes: {}", total_meshes);
    println!("Total Vertices: {}", total_vertices);
    println!("Total Faces: {}", total_faces);
    
    // BMW should have significant geometry
    assert!(total_meshes > 0, "BMW should have meshes");
    assert!(total_vertices > 1000, "BMW should have many vertices");
    
    // Check bounds of first mesh to verify data is reasonable
    fn find_first_mesh(obj: &alembic::abc::IObject) -> Option<(glam::Vec3, glam::Vec3, String)> {
        if let Some(mesh) = IPolyMesh::new(obj) {
            if let Ok(sample) = mesh.getSample(0) {
                if sample.num_vertices() > 0 {
                    let (min, max) = sample.compute_bounds();
                    return Some((min, max, mesh.getName().to_string()));
                }
            }
        }
        for child in obj.getChildren() {
            if let Some(result) = find_first_mesh(&child) {
                return Some(result);
            }
        }
        None
    }
    
    if let Some((min, max, name)) = find_first_mesh(&root) {
        println!("\nFirst mesh '{}' bounds:", name);
        println!("  min: {:?}", min);
        println!("  max: {:?}", max);
        let size = max - min;
        println!("  size: {:?}", size);
        
        // Sanity check - bounds should be reasonable for a car (not NaN, not huge)
        assert!(!min.x.is_nan() && !min.y.is_nan() && !min.z.is_nan());
        assert!(!max.x.is_nan() && !max.y.is_nan() && !max.z.is_nan());
        assert!(size.length() < 10000.0, "Bounds too large - likely garbage data");
    }
}

#[test]
fn test_explore_structure() {
    let archive = OgawaIArchive::open(CHESS3_PATH).expect("Failed to open");
    let root = archive.root();
    
    println!("\n=== Exploring chess3.abc structure ===");
    println!("File size: {} bytes", archive.streams().size());
    
    explore_group(&root, "", 0, 3);
}

fn explore_group(group: &alembic::ogawa::IGroup, prefix: &str, depth: usize, max_depth: usize) {
    if depth >= max_depth {
        return;
    }
    
    let indent = "  ".repeat(depth);
    
    for i in 0..group.num_children() {
        let offset = match group.child_offset(i) {
            Ok(o) => o,
            Err(e) => {
                println!("{}{}{}: Error getting offset: {:?}", indent, prefix, i, e);
                continue;
            }
        };
        
        // Check if group or data
        // In ACTUAL Alembic files: MSB=0 means GROUP, MSB=1 means DATA
        let is_group = (offset & (1u64 << 63)) == 0;
        let actual_offset = offset & !(1u64 << 63);
        
        if is_group {
            if actual_offset == 0 {
                println!("{}{}{}: Empty Group", indent, prefix, i);
            } else {
                match group.group(i) {
                    Ok(child_group) => {
                        println!("{}{}{}: Group @ 0x{:X} ({} children)", 
                                indent, prefix, i, actual_offset, child_group.num_children());
                        explore_group(&child_group, &format!("{}{}/", prefix, i), depth + 1, max_depth);
                    }
                    Err(e) => {
                        println!("{}{}{}: Group @ 0x{:X} - Error: {:?}", 
                                indent, prefix, i, actual_offset, e);
                    }
                }
            }
        } else {
            if actual_offset == 0 {
                println!("{}{}{}: Empty Data", indent, prefix, i);
            } else {
                match group.data(i) {
                    Ok(data) => {
                        println!("{}{}{}: Data @ 0x{:X} ({} bytes)", 
                                indent, prefix, i, actual_offset, data.size());
                        
                        // Try to read and interpret small data blocks
                        if data.size() > 0 && data.size() < 200 {
                            if let Ok(bytes) = data.read_all() {
                                // Check if it looks like a string
                                if bytes.iter().all(|&b| b == 0 || (b >= 32 && b < 127)) {
                                    if let Ok(s) = String::from_utf8(bytes.clone()) {
                                        let s = s.trim_matches('\0');
                                        if !s.is_empty() && s.len() < 100 {
                                            println!("{}     String: \"{}\"", indent, s);
                                        }
                                    }
                                }
                                if bytes.len() <= 64 {
                                    println!("{}     Hex: {:02X?}", indent, &bytes[..bytes.len().min(32)]);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        println!("{}{}{}: Data @ 0x{:X} - Error: {:?}", 
                                indent, prefix, i, actual_offset, e);
                    }
                }
            }
        }
    }
}

#[test]
fn test_high_level_api() {
    let archive = IArchive::open(CHESS3_PATH).expect("Failed to open");
    
    println!("\n=== High-level API ===");
    println!("Archive: {}", archive.getName());
    println!("Time samplings: {}", archive.getNumTimeSamplings());
    
    let root = archive.getTop();
    println!("Root name: '{}'", root.getName());
    println!("Root full_name: '{}'", root.getFullName());
    println!("Root children: {}", root.getNumChildren());
    
    let props = root.getProperties();
    println!("Root properties: {}", props.getNumProperties());
}

#[test]
fn test_traverse_hierarchy() {
    let archive = IArchive::open(CHESS3_PATH).expect("Failed to open");
    let root = archive.getTop();
    
    println!("\n=== Object Hierarchy ===");
    traverse_iobject(&root, 0);
}

fn traverse_iobject(obj: &alembic::abc::IObject, depth: usize) {
    let indent = "  ".repeat(depth);
    
    println!("{}Object: '{}' ({})", indent, obj.getName(), obj.getFullName());
    
    // Show properties
    let props = obj.getProperties();
    if props.getNumProperties() > 0 {
        println!("{}  Properties: {}", indent, props.getNumProperties());
        for name in props.getPropertyNames() {
            println!("{}    - {}", indent, name);
        }
    }
    
    // Show metadata
    let header = obj.getHeader();
    if !header.meta_data.is_empty() {
        println!("{}  Schema: {}", indent, header.meta_data.get("schema").unwrap_or_default());
    }
    
    // Recurse children (limit depth to avoid huge output)
    let num_children = obj.getNumChildren();
    if depth < 2 && num_children > 0 {
        for child in obj.getChildren() {
            traverse_iobject(&child, depth + 1);
        }
    } else if num_children > 0 {
        println!("{}  ... {} more children", indent, num_children);
    }
}

#[test]
fn test_polymesh_schema() {
    let archive = IArchive::open(CHESS3_PATH).expect("Failed to open");
    let root = archive.getTop();
    
    println!("\n=== Testing IPolyMesh ===");
    
    // Find polymesh objects (they're children of xforms)
    let mut mesh_count = 0;
    for child in root.getChildren() {
        // Look for mesh children of xform objects
        for grandchild in child.getChildren() {
            if grandchild.matchesSchema(POLYMESH_SCHEMA) {
                mesh_count += 1;
                if mesh_count <= 3 {
                    println!("Found PolyMesh: '{}'", grandchild.getName());
                    
                    if let Some(mesh) = IPolyMesh::new(&grandchild) {
                        println!("  - IPolyMesh created successfully");
                        println!("  - Name: {}", mesh.getName());
                        println!("  - Full name: {}", mesh.getFullName());
                        println!("  - Constant: {}", mesh.isConstant());
                        println!("  - Properties: {:?}", mesh.getPropertyNames());
                    }
                }
            }
        }
    }
    
    println!("Total PolyMesh objects: {}", mesh_count);
    assert!(mesh_count > 0, "Should find at least one PolyMesh");
}

#[test]
fn test_xform_schema() {
    let archive = IArchive::open(CHESS3_PATH).expect("Failed to open");
    let root = archive.getTop();
    
    println!("\n=== Testing IXform ===");
    
    // Find first xform object
    let mut xform_count = 0;
    for child in root.getChildren() {
        if child.matchesSchema(XFORM_SCHEMA) {
            xform_count += 1;
            if xform_count <= 3 {
                println!("Found Xform: '{}'", child.getName());
                
                // Try to create IXform
                if let Some(xform) = IXform::new(&child) {
                    println!("  - IXform created successfully");
                    println!("  - Name: {}", xform.getName());
                    println!("  - Full name: {}", xform.getFullName());
                    println!("  - Inheriting: {}", xform.is_inheriting());
                    println!("  - Constant: {}", xform.isConstant());
                }
            }
        }
    }
    
    println!("Total Xform objects: {}", xform_count);
    assert!(xform_count > 0, "Should find at least one Xform");
}

#[test]
fn test_read_alembic_header_data() {
    // In Alembic/Ogawa, the first children of root contain archive metadata
    let archive = OgawaIArchive::open(CHESS3_PATH).expect("Failed to open");
    let root = archive.root();
    
    println!("\n=== Reading Alembic header ===");
    
    // Root should have children
    assert!(root.num_children() > 0);
    
    // Explore first few children
    for i in 0..root.num_children().min(3) {
        println!("\n--- Root child {} ---", i);
        
        if root.is_child_group(i).unwrap_or(false) {
            match root.group(i) {
                Ok(group) => {
                    println!("Group with {} children", group.num_children());
                    
                    // Explore nested children
                    for j in 0..group.num_children().min(5) {
                        if let Ok(true) = group.is_child_group(j) {
                            if let Ok(subgroup) = group.group(j) {
                                println!("  [{}]: Subgroup with {} children", j, subgroup.num_children());
                            }
                        } else if let Ok(true) = group.is_child_data(j) {
                            if let Ok(data) = group.data(j) {
                                println!("  [{}]: Data, {} bytes", j, data.size());
                                
                                // Try to read small data
                                if data.size() > 0 && data.size() < 200 {
                                    if let Ok(bytes) = data.read_all() {
                                        // Check if string-like
                                        if bytes.iter().all(|&b| b == 0 || (b >= 32 && b < 127)) {
                                            if let Ok(s) = String::from_utf8(bytes.clone()) {
                                                let s = s.trim_matches('\0');
                                                if !s.is_empty() {
                                                    println!("       Text: \"{}\"", s);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => println!("Error reading group: {:?}", e),
            }
        } else if root.is_child_data(i).unwrap_or(false) {
            match root.data(i) {
                Ok(data) => println!("Data: {} bytes", data.size()),
                Err(e) => println!("Error reading data: {:?}", e),
            }
        }
    }
}

#[test]
fn test_xform_get_sample() {
    let archive = IArchive::open(CHESS3_PATH).expect("Failed to open");
    let root = archive.getTop();
    
    println!("\n=== Testing IXform.get_sample() ===");
    
    // Find first xform and read its sample
    let mut found_xform = false;
    
    for child in root.getChildren() {
        if let Some(xform) = IXform::new(&child) {
            found_xform = true;
            println!("Found xform: '{}'", xform.getName());
            println!("  properties: {:?}", xform.object().getProperties().getPropertyNames());
            println!("  num_samples: {}", xform.getNumSamples());
            println!("  is_inheriting: {}", xform.is_inheriting());
            println!("  is_constant: {}", xform.isConstant());
            
            // Check .xform compound contents
            let props = xform.object().getProperties();
            if let Some(xf_prop) = props.getPropertyByName(".xform") {
                if let Some(xf_compound) = xf_prop.asCompound() {
                    println!("  .xform sub-properties: {:?}", xf_compound.getPropertyNames());
                }
            }
            
            // Read sample 0
            match xform.getSample(0) {
                Ok(sample) => {
                    println!("  Sample 0:");
                    println!("    ops count: {}", sample.ops.len());
                    println!("    inherits: {}", sample.inherits);
                    
                    for (i, op) in sample.ops.iter().enumerate() {
                        println!("    op[{}]: {:?} = {:?}", i, op.op_type, op.values);
                    }
                    
                    // Compute and display matrix
                    let matrix = sample.matrix();
                    let translation = sample.translation();
                    let scale = sample.scale();
                    
                    println!("    translation: {:?}", translation);
                    println!("    scale: {:?}", scale);
                    println!("    matrix row0: {:?}", matrix.row(0));
                }
                Err(e) => {
                    println!("  Error reading sample: {:?}", e);
                }
            }
            
            // Only test first xform
            break;
        }
    }
    
    assert!(found_xform, "Should find at least one Xform");
}

#[test]
fn test_polymesh_get_sample() {
    let archive = IArchive::open(CHESS3_PATH).expect("Failed to open");
    let root = archive.getTop();
    
    println!("\n=== Testing IPolyMesh.get_sample() ===");
    
    // Find first polymesh and read its sample
    let mut found_mesh = false;
    
    for child in root.getChildren() {
        for grandchild in child.getChildren() {
            if let Some(mesh) = IPolyMesh::new(&grandchild) {
                found_mesh = true;
                println!("Found mesh: '{}'", mesh.getName());
                println!("  num_samples: {}", mesh.getNumSamples());
                
                // Read sample 0
                match mesh.getSample(0) {
                    Ok(sample) => {
                        println!("  Sample 0:");
                        println!("    vertices: {}", sample.num_vertices());
                        println!("    faces: {}", sample.num_faces());
                        println!("    indices: {}", sample.num_indices());
                        println!("    has_normals: {}", sample.has_normals());
                        println!("    has_uvs: {}", sample.has_uvs());
                        println!("    is_valid: {}", sample.is_valid());
                        
                        // Validate data
                        assert!(sample.is_valid(), "Sample should be valid");
                        assert!(sample.num_vertices() > 0, "Should have vertices");
                        assert!(sample.num_faces() > 0, "Should have faces");
                        
                        // Compute bounds
                        let (min, max) = sample.compute_bounds();
                        println!("    bounds: min={:?}, max={:?}", min, max);
                    }
                    Err(e) => {
                        println!("  Error reading sample: {:?}", e);
                    }
                }
                
                // Only test first mesh
                break;
            }
        }
        if found_mesh { break; }
    }
    
    assert!(found_mesh, "Should find at least one PolyMesh");
}

#[test]
fn test_read_mesh_properties() {
    let archive = IArchive::open(CHESS3_PATH).expect("Failed to open");
    let root = archive.getTop();
    
    println!("\n=== Reading Mesh Properties ===");
    
    // Find first polymesh and read its properties
    let mut found_mesh = false;
    
    for child in root.getChildren() {
        for grandchild in child.getChildren() {
            if grandchild.matchesSchema(POLYMESH_SCHEMA) {
                found_mesh = true;
                println!("Found PolyMesh: '{}'", grandchild.getName());
                
                // Get properties
                let props = grandchild.getProperties();
                println!("  Properties: {:?}", props.getPropertyNames());
                
                // Try to read .geom compound which contains mesh data
                if let Some(geom_prop) = props.getPropertyByName(".geom") {
                    println!("  Found .geom property");
                    
                    if let Some(compound) = geom_prop.asCompound() {
                        println!("    .geom sub-properties: {:?}", compound.getPropertyNames());
                        
                        // Try to read P (positions)
                        if let Some(p_prop) = compound.getPropertyByName("P") {
                            println!("    Found P property");
                            if let Some(array_reader) = p_prop.asArray() {
                                let num_samples = array_reader.getNumSamples();
                                println!("      P num_samples: {}", num_samples);
                                
                                if num_samples > 0 {
                                    match array_reader.getSampleLen(0) {
                                        Ok(len) => println!("      P sample[0] len: {} elements", len),
                                        Err(e) => println!("      P sample_len error: {:?}", e),
                                    }
                                    
                                    match array_reader.getSampleVec(0) {
                                        Ok(data) => {
                                            println!("      P sample[0] bytes: {} bytes", data.len());
                                            let num_verts = data.len() / 12; // 3 floats * 4 bytes
                                            println!("      P sample[0] vertices: {}", num_verts);
                                            
                                            // Print first few vertices
                                            if data.len() >= 12 {
                                                let floats: &[f32] = bytemuck::cast_slice(&data);
                                                for i in 0..floats.len().min(9) / 3 {
                                                    println!("        v{}: ({:.3}, {:.3}, {:.3})", 
                                                        i, floats[i*3], floats[i*3+1], floats[i*3+2]);
                                                }
                                            }
                                        }
                                        Err(e) => println!("      P read error: {:?}", e),
                                    }
                                }
                            }
                        }
                        
                        // Try to read .faceCounts
                        if let Some(fc_prop) = compound.getPropertyByName(".faceCounts") {
                            println!("    Found .faceCounts property");
                            if let Some(array_reader) = fc_prop.asArray() {
                                println!("      .faceCounts num_samples: {}", array_reader.getNumSamples());
                            }
                        }
                        
                        // Try to read .faceIndices
                        if let Some(fi_prop) = compound.getPropertyByName(".faceIndices") {
                            println!("    Found .faceIndices property");
                            if let Some(array_reader) = fi_prop.asArray() {
                                println!("      .faceIndices num_samples: {}", array_reader.getNumSamples());
                            }
                        }
                    }
                }
                
                // Only process first mesh for this test
                break;
            }
        }
        if found_mesh { break; }
    }
    
    assert!(found_mesh, "Should find at least one PolyMesh");
}
