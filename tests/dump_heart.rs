//! Dump heart.abc structure to understand what we need to copy

use alembic::abc::IArchive;
use alembic::core::PropertyType;
use std::path::Path;

fn dump_properties_deep(props: &alembic::abc::ICompoundProperty, indent: usize) {
    let prefix = "  ".repeat(indent);
    
    for i in 0..props.getNumProperties() {
        if let Some(header) = props.getPropertyHeader(i) {
            let type_str = match header.property_type {
                PropertyType::Compound => "Compound",
                PropertyType::Scalar => "Scalar",
                PropertyType::Array => "Array",
            };
            
            println!("{}[{}] {} - type={}, pod={:?}, extent={}, ts_idx={}", 
                prefix, i, header.name,
                type_str, header.data_type.pod, header.data_type.extent,
                header.time_sampling_index);
            println!("{}     meta: [{}]", prefix, header.meta_data.serialize());
            
            // Get the property to show more details
            if let Some(prop) = props.getProperty(i) {
                if prop.isCompound() {
                    if let Some(child_compound) = prop.asCompound() {
                        println!("{}     -> {} sub-properties", prefix, child_compound.getNumProperties());
                        dump_properties_deep(&child_compound, indent + 1);
                    }
                }
            }
        }
    }
}

fn dump_object_deep(obj: &alembic::abc::IObject, indent: usize) {
    let prefix = "  ".repeat(indent);
    
    let header = obj.getHeader();
    println!("{}Object: {}", prefix, obj.getName());
    println!("{}  meta: [{}]", prefix, header.meta_data.serialize());
    
    let props = obj.getProperties();
    println!("{}  Properties ({}):", prefix, props.getNumProperties());
    dump_properties_deep(&props, indent + 2);
    
    // Recurse children
    for child in obj.getChildren() {
        dump_object_deep(&child, indent + 1);
    }
}

#[test]
fn test_dump_heart_structure() {
    let path = "data/Abc/heart.abc";
    if !Path::new(path).exists() {
        eprintln!("Skipping: {} not found", path);
        return;
    }
    
    let archive = IArchive::open(path).expect("Failed to open");
    println!("\n=== HEART.ABC DETAILED STRUCTURE ===\n");
    println!("Time samplings: {}", archive.getNumTimeSamplings());
    for i in 0..archive.getNumTimeSamplings() {
        if let Some(ts) = archive.getTimeSampling(i) {
            println!("  TS[{}]: {:?}", i, ts);
        }
    }
    println!();
    
    let root = archive.getTop();
    dump_object_deep(&root, 0);
}
