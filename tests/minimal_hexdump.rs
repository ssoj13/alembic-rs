//! Minimal hex dump test for binary analysis

use alembic::ogawa::writer::{OArchive, OObject};
use std::fs::File;
use std::io::Read;

#[test]
fn test_minimal_hexdump() {
    let path = std::env::temp_dir().join("minimal_hexdump.abc");
    
    // Minimal: root with one empty child
    {
        let mut archive = OArchive::create(&path).unwrap();
        let mut root = OObject::new("");
        root.add_child(OObject::new("child1"));
        archive.write_archive(&root).unwrap();
    }
    
    // Hex dump first 256 bytes
    let mut f = File::open(&path).unwrap();
    let mut buf = vec![0u8; 256];
    let read = f.read(&mut buf).unwrap();
    buf.truncate(read);
    
    println!("\n=== First {} bytes of minimal.abc ===", buf.len());
    for (i, chunk) in buf.chunks(16).enumerate() {
        print!("{:04x}: ", i * 16);
        for b in chunk {
            print!("{:02x} ", b);
        }
        // ASCII
        print!(" |");
        for b in chunk {
            let c = if *b >= 0x20 && *b < 0x7f { *b as char } else { '.' };
            print!("{}", c);
        }
        println!("|");
    }
    
    // Annotate structure
    println!("\n=== Structure ===");
    println!("0x00-0x04: Magic = {:?}", String::from_utf8_lossy(&buf[0..5]));
    println!("0x05: Frozen = 0x{:02x}", buf[5]);
    println!("0x06-0x07: Version = 0x{:02x}{:02x}", buf[6], buf[7]);
    let root_pos = u64::from_le_bytes(buf[8..16].try_into().unwrap());
    println!("0x08-0x0F: Root position = 0x{:x} ({})", root_pos, root_pos);
    
    // After header (16 bytes)
    println!("0x10: Data starts");
}
