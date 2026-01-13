//! Compare Rust output with C++ reference file

use alembic::ogawa::writer::{OArchive, OObject};
use std::fs::File;
use std::io::Read;

#[test]
fn test_compare_with_cpp() {
    let rust_path = std::env::temp_dir().join("rust_minimal.abc");
    let cpp_path = r"C:\projects\projects.rust\_done\alembic-rs\test_cpp\minimal_cpp.abc";
    
    // Create minimal archive matching C++ test
    {
        let mut archive = OArchive::create(&rust_path).unwrap();
        let mut root = OObject::new("");
        root.add_child(OObject::new("child1"));
        archive.write_archive(&root).unwrap();
    }
    
    // Read both files
    let mut rust_file = File::open(&rust_path).unwrap();
    let mut cpp_file = File::open(cpp_path).unwrap();
    
    let mut rust_data = Vec::new();
    let mut cpp_data = Vec::new();
    
    rust_file.read_to_end(&mut rust_data).unwrap();
    cpp_file.read_to_end(&mut cpp_data).unwrap();
    
    println!("\n=== File sizes ===");
    println!("Rust: {} bytes", rust_data.len());
    println!("C++:  {} bytes", cpp_data.len());
    
    // Print hex dump side by side
    println!("\n=== Byte-by-byte comparison (first 256 bytes) ===");
    println!("Offset  C++             Rust            Diff");
    println!("------  --------------  --------------  ----");
    
    let max_len = std::cmp::max(rust_data.len(), cpp_data.len()).min(256);
    let mut diff_count = 0;
    let mut diff_ranges: Vec<(usize, usize)> = Vec::new();
    let mut in_diff = false;
    let mut diff_start = 0;
    
    for i in 0..max_len {
        let cpp_byte = cpp_data.get(i).copied().unwrap_or(0);
        let rust_byte = rust_data.get(i).copied().unwrap_or(0);
        
        if cpp_byte != rust_byte {
            if !in_diff {
                in_diff = true;
                diff_start = i;
            }
            diff_count += 1;
            
            if i < 256 {
                println!("0x{:04x}: {:02x}              {:02x}              <-- DIFF", i, cpp_byte, rust_byte);
            }
        } else {
            if in_diff {
                diff_ranges.push((diff_start, i - 1));
                in_diff = false;
            }
        }
    }
    if in_diff {
        diff_ranges.push((diff_start, max_len - 1));
    }
    
    println!("\n=== Summary ===");
    println!("Total different bytes in first 256: {}", diff_count);
    println!("Different ranges: {:?}", diff_ranges);
    
    // Analyze key regions
    println!("\n=== Key regions analysis ===");
    
    // Header comparison
    println!("Magic (0x00-0x04): C++={:?} Rust={:?}", 
        String::from_utf8_lossy(&cpp_data[0..5]), 
        String::from_utf8_lossy(&rust_data[0..5]));
    
    // Root position
    let cpp_root = u64::from_le_bytes(cpp_data[8..16].try_into().unwrap());
    let rust_root = u64::from_le_bytes(rust_data[8..16].try_into().unwrap());
    println!("Root position (0x08-0x0F): C++={:#x} Rust={:#x} diff={}", 
        cpp_root, rust_root, cpp_root as i64 - rust_root as i64);
        
    // Child hash comparison at 0x7B-0x9A
    println!("\nData hash (0x7B-0x8A):");
    println!("  C++:  {:02x?}", &cpp_data[0x7B..0x8B]);
    println!("  Rust: {:02x?}", &rust_data[0x7B..0x8B]);
    
    println!("\nChild hash (0x8B-0x9A):");
    println!("  C++:  {:02x?}", &cpp_data[0x8B..0x9B]);
    println!("  Rust: {:02x?}", &rust_data[0x8B..0x9B]);
}
