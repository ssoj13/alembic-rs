//! Binary parity test: copy heart.abc and compare with original.
//! 
//! This test validates binary parity between original Alembic files and our output.

use alembic::abc::{IArchive, ICompoundProperty, IProperty};
use alembic::ogawa::writer::{
    OArchive, OObject, OPolyMesh, OPolyMeshSample, OProperty, OPropertyData, OXform, OXformSample,
};
use alembic::geom::{IXform, IPolyMesh, XFORM_SCHEMA, POLYMESH_SCHEMA};
use alembic::util::PlainOldDataType;
use std::path::Path;
use std::collections::HashMap;

const HEART_PATH: &str = "data/Abc/heart.abc";

fn copy_time_samplings(src: &IArchive, dst: &mut OArchive) -> HashMap<u32, u32> {
    let mut mapping = HashMap::new();
    mapping.insert(0, 0);
    
    for i in 1..src.getNumTimeSamplings() {
        if let Some(ts) = src.getTimeSampling(i) {
            let new_idx = dst.addTimeSampling(ts.clone());
            mapping.insert(i as u32, new_idx);
        }
    }
    
    mapping
}

fn map_ts(ts_map: &HashMap<u32, u32>, src_idx: u32) -> u32 {
    *ts_map.get(&src_idx).unwrap_or(&src_idx)
}

fn merge_property(target: &mut Vec<OProperty>, prop: OProperty) {
    if let Some(existing) = target.iter_mut().find(|p| p.name == prop.name) {
        match (&mut existing.data, prop.data) {
            (OPropertyData::Compound(dst_children), OPropertyData::Compound(src_children)) => {
                for child in src_children {
                    merge_property(dst_children, child);
                }
            }
            _ => {}
        }
        return;
    }
    target.push(prop);
}

fn copy_property_recursive(prop: &IProperty<'_>, ts_map: &HashMap<u32, u32>) -> Option<OProperty> {
    let header = prop.getHeader();
    let name = &header.name;
    let data_type = header.data_type;
    let ts_idx = map_ts(ts_map, header.time_sampling_index);
    let meta = header.meta_data.clone();

    if let Some(compound) = prop.asCompound() {
        let mut out = OProperty::compound(name);
        out.meta_data = meta;
        out.time_sampling_index = ts_idx;
        let num = compound.getNumProperties();
        for i in 0..num {
            if let Some(child) = compound.getProperty(i) {
                if let Some(out_child) = copy_property_recursive(&child, ts_map) {
                    out.add_child(out_child);
                }
            }
        }
        return Some(out);
    }

    if let Some(scalar) = prop.asScalar() {
        let mut out = OProperty::scalar(name, data_type);
        out.meta_data = meta;
        out.time_sampling_index = ts_idx;
        let num_samples = scalar.getNumSamples();
        let is_string = matches!(
            data_type.pod,
            PlainOldDataType::String | PlainOldDataType::Wstring
        );
        for i in 0..num_samples {
            if is_string {
                if let Ok(buf) = scalar.getSampleVec(i) {
                    out.add_scalar_sample(&buf);
                }
            } else {
                let sample_size = data_type.num_bytes();
                let mut buf = vec![0u8; sample_size];
                if scalar.getSample(i, &mut buf).is_ok() {
                    out.add_scalar_sample(&buf);
                }
            }
        }
        return Some(out);
    }

    if let Some(array) = prop.asArray() {
        let mut out = OProperty::array(name, data_type);
        out.meta_data = meta;
        out.time_sampling_index = ts_idx;
        let num_samples = array.getNumSamples();
        for i in 0..num_samples {
            if let (Ok(data), Ok(dims)) = (array.getSampleVec(i), array.getDimensions(i)) {
                out.add_array_sample(&data, &dims);
            }
        }
        return Some(out);
    }

    None
}

fn copy_properties_from(
    props: &ICompoundProperty<'_>,
    out_props: &mut Vec<OProperty>,
    ts_map: &HashMap<u32, u32>,
) {
    let num_props = props.getNumProperties();
    for i in 0..num_props {
        if let Some(prop) = props.getProperty(i) {
            if let Some(out_prop) = copy_property_recursive(&prop, ts_map) {
                merge_property(out_props, out_prop);
            }
        }
    }
}

/// Copy root object properties (Maya metadata: .childBnds, statistics, N.samples)
fn copy_root_properties(
    root: &alembic::abc::IObject,
    out_root: &mut OObject,
    ts_map: &HashMap<u32, u32>,
) {
    let props = root.getProperties();
    copy_properties_from(&props, &mut out_root.properties, ts_map);
}

fn convert_object(
    obj: &alembic::abc::IObject,
    _src_archive: &IArchive,
    ts_map: &HashMap<u32, u32>,
) -> OObject {
    let mut out = OObject::new(obj.getName());
    let header = obj.getHeader();
    out.meta_data = header.meta_data.clone();
    
    // XFORM
    if obj.matchesSchema(XFORM_SCHEMA) {
        if let Some(xform) = IXform::new(obj) {
            let num_samples = xform.getNumSamples();
            let mut oxform = OXform::new(obj.getName());
            
            let src_ts_idx = xform.getTimeSamplingIndex();
            let dst_ts_idx = *ts_map.get(&src_ts_idx).unwrap_or(&0);
            oxform.set_time_sampling(dst_ts_idx);
            
            for i in 0..num_samples {
                if let Ok(sample) = xform.getSample(i) {
                    // Copy ops directly to preserve identity xforms (empty ops)
                    // Using from_matrix() would create a Matrix op even for identity!
                    println!("Xform sample {}: {} ops, inherits={}", i, sample.ops.len(), sample.inherits);
                    for (j, op) in sample.ops.iter().enumerate() {
                        println!("  Op {}: {:?}, {} values", j, op.op_type, op.values.len());
                    }
                    oxform.add_sample(OXformSample::from_ops(sample.ops.clone(), sample.inherits));
                }
            }
            let built = oxform.build();
            out.meta_data = built.meta_data;
            out.properties = built.properties;
        }
    }
    
    // POLYMESH
    if obj.matchesSchema(POLYMESH_SCHEMA) {
        if let Some(mesh) = IPolyMesh::new(obj) {
            let num_samples = mesh.getNumSamples();
            let mut omesh = OPolyMesh::new(obj.getName());
            
            let src_ts_idx = mesh.getTimeSamplingIndex();
            let dst_ts_idx = *ts_map.get(&src_ts_idx).unwrap_or(&0);
            omesh.set_time_sampling(dst_ts_idx);
            
            for i in 0..num_samples {
                if let Ok(sample) = mesh.getSample(i) {
                    println!("PolyMesh sample {}: verts={}, normals={:?} (simple={}), uvs={:?}",
                        i, sample.num_vertices(),
                        sample.normals.as_ref().map(|n| n.len()),
                        sample.normals_is_simple_array,
                        sample.uvs.as_ref().map(|u| u.len()));
                    if sample.num_vertices() > 0 {
                        let mut out_sample = OPolyMeshSample::new(
                            sample.positions.clone(),
                            sample.face_counts.clone(),
                            sample.face_indices.clone(),
                        );
                        out_sample.velocities = sample.velocities.clone();
                        out_sample.normals = sample.normals.clone();
                        out_sample.uvs = sample.uvs.clone();
                        omesh.add_sample(&out_sample);
                    }
                }
            }
            let built = omesh.build();
            out.meta_data = built.meta_data;
            out.properties = built.properties;
        }
    }

    copy_properties_from(&obj.getProperties(), &mut out.properties, ts_map);
    
    // Recurse for children
    for child in obj.getChildren() {
        out.children.push(convert_object(&child, _src_archive, ts_map));
    }
    
    out
}

#[test]
fn test_copy_heart_binary_parity() {
    if !Path::new(HEART_PATH).exists() {
        eprintln!("Skipping test: {} not found", HEART_PATH);
        return;
    }
    
    let output_path = "test_heart_parity.abc";
    
    // Read and convert
    let original = IArchive::open(HEART_PATH).expect("Failed to open heart.abc");
    let orig_root = original.getTop();
    
    {
        let mut archive = OArchive::create(output_path).expect("Failed to create archive");
        
        // Copy archive metadata
        archive.set_archive_metadata(original.getArchiveMetaData().clone());
        archive.set_library_version(original.getArchiveVersion());
        archive.set_indexed_metadata(original.getIndexedMetaData());
        
        let ts_map = copy_time_samplings(&original, &mut archive);
        
        let mut out_root = OObject::new("");
        
        // Copy root object properties (Maya metadata: .childBnds, statistics, N.samples)
        copy_root_properties(&orig_root, &mut out_root, &ts_map);
        
        for child in orig_root.getChildren() {
            out_root.children.push(convert_object(&child, &original, &ts_map));
        }
        
        archive.write_archive(&out_root).expect("Failed to write archive");
    }
    
    // Compare binary files
    let src_bytes = std::fs::read(HEART_PATH).expect("Failed to read source");
    let dst_bytes = std::fs::read(output_path).expect("Failed to read dest");
    
    println!("\n=== BINARY PARITY TEST: heart.abc ===");
    println!("Source size: {} bytes", src_bytes.len());
    println!("Output size: {} bytes", dst_bytes.len());
    
    let min_len = src_bytes.len().min(dst_bytes.len());
    let mut diff_count = 0;
    let mut diff_regions: Vec<(usize, usize)> = Vec::new();
    let mut in_diff = false;
    let mut diff_start = 0;
    
    for i in 0..min_len {
        if src_bytes[i] != dst_bytes[i] {
            if !in_diff {
                in_diff = true;
                diff_start = i;
            }
            diff_count += 1;
        } else if in_diff {
            diff_regions.push((diff_start, i));
            in_diff = false;
        }
    }
    if in_diff {
        diff_regions.push((diff_start, min_len));
    }
    
    // Size difference
    let size_diff = src_bytes.len().abs_diff(dst_bytes.len());
    diff_count += size_diff;
    
    let max_len = src_bytes.len().max(dst_bytes.len());
    let match_pct = if max_len > 0 {
        ((max_len - diff_count) as f64 / max_len as f64) * 100.0
    } else {
        100.0
    };
    
    println!("Binary match: {:.2}%", match_pct);
    println!("Different bytes: {}", diff_count);
    println!("Difference regions: {}", diff_regions.len());
    
    // Show first few diff regions
    for (i, (start, end)) in diff_regions.iter().take(5).enumerate() {
        println!("  Region {}: 0x{:04x} - 0x{:04x} ({} bytes)", i+1, start, end, end - start);
        
        // Show actual bytes
        let show_start = start.saturating_sub(4);
        let show_end = (*end + 4).min(min_len);
        print!("    Orig: ");
        for j in show_start..show_end {
            if j >= *start && j < *end {
                print!("[{:02x}]", src_bytes[j]);
            } else {
                print!(" {:02x} ", src_bytes[j]);
            }
        }
        println!();
        print!("    Ours: ");
        for j in show_start..show_end {
            if j >= *start && j < *end {
                print!("[{:02x}]", dst_bytes[j]);
            } else {
                print!(" {:02x} ", dst_bytes[j]);
            }
        }
        println!();
    }
    
    // Don't delete for manual inspection
    println!("\nOutput file: {}", output_path);
    
    assert!(match_pct >= 99.0, "Binary parity too low: {:.2}%", match_pct);
}
