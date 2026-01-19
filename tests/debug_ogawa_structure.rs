//! Debug tool to compare Ogawa file structures.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

const HEART_PATH: &str = "data/Abc/heart.abc";
const OUTPUT_PATH: &str = "test_heart_parity.abc";

fn read_u64(file: &mut File) -> u64 {
    let mut buf = [0u8; 8];
    file.read_exact(&mut buf).unwrap();
    u64::from_le_bytes(buf)
}

fn is_data_offset(val: u64) -> bool {
    (val & 0x8000_0000_0000_0000) != 0
}

fn extract_offset(val: u64) -> u64 {
    val & 0x7FFF_FFFF_FFFF_FFFF
}

fn dump_group(file: &mut File, pos: u64, indent: &str, label: &str) {
    file.seek(SeekFrom::Start(pos)).unwrap();
    let count = read_u64(file);
    println!("{}[0x{:04x}] {} (count={})", indent, pos, label, count);
    
    for i in 0..count {
        let child = read_u64(file);
        let is_data = is_data_offset(child);
        let offset = extract_offset(child);
        let marker = if is_data { "D" } else { "G" };
        println!("{}  [{}] child {}: 0x{:04x} ({})", indent, marker, i, offset, 
            if is_data { "data" } else { "group" });
    }
}

fn dump_data(file: &mut File, pos: u64, indent: &str, label: &str) {
    if pos == 0 {
        println!("{}[0x0000] {} (empty)", indent, label);
        return;
    }
    file.seek(SeekFrom::Start(pos)).unwrap();
    let size = read_u64(file);
    let mut data = vec![0u8; size.min(64) as usize];
    file.read_exact(&mut data).unwrap();
    
    let hex: String = data.iter().take(32).map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join(" ");
    println!("{}[0x{:04x}] {} (size={}) => {}{}", 
        indent, pos, label, size, hex,
        if size > 32 { "..." } else { "" });
}

fn analyze_archive(path: &str) {
    println!("\n=== Analyzing: {} ===", path);
    
    let mut file = File::open(path).unwrap();
    
    // Header
    let mut magic = [0u8; 5];
    file.read_exact(&mut magic).unwrap();
    let frozen = {
        let mut b = [0u8; 1];
        file.read_exact(&mut b).unwrap();
        b[0]
    };
    let version = {
        let mut b = [0u8; 2];
        file.read_exact(&mut b).unwrap();
        u16::from_be_bytes(b)
    };
    let root_pos = read_u64(&mut file);
    
    println!("Magic: {:?}", String::from_utf8_lossy(&magic));
    println!("Frozen: 0x{:02x}", frozen);
    println!("Version: {}", version);
    println!("Root pos: 0x{:04x}", root_pos);
    
    // Root group
    dump_group(&mut file, root_pos, "", "ROOT");
    
    // Get root children
    file.seek(SeekFrom::Start(root_pos)).unwrap();
    let count = read_u64(&mut file);
    let mut children = Vec::new();
    for _ in 0..count {
        children.push(read_u64(&mut file));
    }
    
    // Parse root children
    // 0: version data
    // 1: library version data  
    // 2: top object group
    // 3: archive metadata
    // 4: time samplings
    // 5: indexed metadata
    
    if children.len() >= 3 {
        dump_data(&mut file, extract_offset(children[0]), "  ", "version");
        dump_data(&mut file, extract_offset(children[1]), "  ", "lib_version");
        
        let top_obj_pos = extract_offset(children[2]);
        dump_group(&mut file, top_obj_pos, "  ", "TOP_OBJECT");
        
        // Get top object children
        file.seek(SeekFrom::Start(top_obj_pos)).unwrap();
        let top_count = read_u64(&mut file);
        let mut top_children = Vec::new();
        for _ in 0..top_count {
            top_children.push(read_u64(&mut file));
        }
        
        // First child should be properties group
        if !top_children.is_empty() {
            let props_pos = extract_offset(top_children[0]);
            if !is_data_offset(top_children[0]) {
                dump_group(&mut file, props_pos, "    ", "PROPS_GROUP");
                
                // Get props children
                file.seek(SeekFrom::Start(props_pos)).unwrap();
                let props_count = read_u64(&mut file);
                let mut props_children = Vec::new();
                for _ in 0..props_count {
                    props_children.push(read_u64(&mut file));
                }
                
                // Dump each property group
                for (i, &child) in props_children.iter().enumerate() {
                    let child_pos = extract_offset(child);
                    if is_data_offset(child) {
                        dump_data(&mut file, child_pos, "      ", &format!("prop_{}_headers", i));
                    } else {
                        dump_group(&mut file, child_pos, "      ", &format!("prop_{}", i));
                    }
                }
            }
        }
        
        // Child objects
        for (i, &child) in top_children.iter().enumerate().skip(1) {
            let child_pos = extract_offset(child);
            if is_data_offset(child) {
                dump_data(&mut file, child_pos, "    ", &format!("child_{}_data", i));
            } else {
                dump_group(&mut file, child_pos, "    ", &format!("CHILD_OBJ_{}", i));
            }
        }
        
        if children.len() > 3 {
            dump_data(&mut file, extract_offset(children[3]), "  ", "archive_meta");
        }
        if children.len() > 4 {
            dump_data(&mut file, extract_offset(children[4]), "  ", "time_samplings");
        }
        if children.len() > 5 {
            dump_data(&mut file, extract_offset(children[5]), "  ", "indexed_meta");
        }
    }
}

#[test]
fn debug_compare_structures() {
    analyze_archive(HEART_PATH);
    analyze_archive(OUTPUT_PATH);
}
