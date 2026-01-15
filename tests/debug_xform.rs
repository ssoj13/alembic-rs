//! Debug test to examine raw xform data

use alembic::abc::IArchive;

#[test]
fn debug_brake_disc_xform() {
    let archive = IArchive::open("data/Abc/bmw.abc").expect("Failed to open archive");
    let root = archive.getTop();
    
    println!("\n=== Examining wheel_lb/moving/brake_disc ===\n");
    
    // Navigate: bmw3 -> wheels -> wheel_lb -> moving -> brake_disc
    let bmw3 = root.getChild(0).expect("bmw3");
    println!("Found: {}", bmw3.getName());
    
    let mut wheels = None;
    for i in 0..bmw3.getNumChildren() {
        if let Some(c) = bmw3.getChild(i) {
            if c.getName() == "wheels" {
                wheels = Some(c);
                break;
            }
        }
    }
    let wheels = wheels.expect("wheels");
    println!("Found: {}", wheels.getName());
    
    let mut wheel_lb = None;
    for i in 0..wheels.getNumChildren() {
        if let Some(c) = wheels.getChild(i) {
            if c.getName() == "wheel_lb" {
                wheel_lb = Some(c);
                break;
            }
        }
    }
    let wheel_lb = wheel_lb.expect("wheel_lb");
    println!("Found: {}", wheel_lb.getName());
    
    let mut moving = None;
    for i in 0..wheel_lb.getNumChildren() {
        if let Some(c) = wheel_lb.getChild(i) {
            if c.getName() == "moving" {
                moving = Some(c);
                break;
            }
        }
    }
    let moving = moving.expect("moving");
    println!("Found: {}", moving.getName());
    
    let mut brake_disc = None;
    for i in 0..moving.getNumChildren() {
        if let Some(c) = moving.getChild(i) {
            if c.getName() == "brake_disc" {
                brake_disc = Some(c);
                break;
            }
        }
    }
    let brake_disc = brake_disc.expect("brake_disc");
    println!("Found: {}", brake_disc.getName());
    
    // Now read raw xform properties
    let props = brake_disc.getProperties();
    
    // Get .xform compound
    let xform_prop = props.property_by_name(".xform").expect(".xform property");
    let xform = xform_prop.as_compound().expect(".xform compound");
    
    // Read .ops
    {
        let ops_prop = xform.property_by_name(".ops").expect(".ops");
        let scalar = ops_prop.as_scalar().expect(".ops scalar");
        let extent = scalar.header().data_type.extent as usize;
        println!("\n.ops extent: {}", extent);
        let mut buf = vec![0u8; extent];
        scalar.read_sample(0, &mut buf).expect("read .ops");
        println!(".ops raw bytes: {:?}", buf);
        println!(".ops decoded:");
        for (i, &byte) in buf.iter().enumerate() {
            let op_type = byte >> 4;
            let hint = byte & 0x0F;
            let type_name = match op_type {
                0 => "Scale",
                1 => "Translate",
                2 => "Rotate",
                3 => "Matrix",
                4 => "RotateX",
                5 => "RotateY",
                6 => "RotateZ",
                _ => "Unknown",
            };
            let hint_name = if op_type == 1 {
                match hint {
                    0 => "TranslateHint",
                    1 => "ScalePivotPoint",
                    2 => "ScalePivotTranslation",
                    3 => "RotatePivotPoint",
                    4 => "RotatePivotTranslation",
                    _ => "Unknown",
                }
            } else {
                "N/A"
            };
            println!("  [{}] byte=0x{:02X} type={} hint={} ({})", 
                i, byte, type_name, hint, hint_name);
        }
    }
    
    // Read .vals
    {
        let vals_prop = xform.property_by_name(".vals").expect(".vals");
        if vals_prop.is_scalar() {
            let scalar = vals_prop.as_scalar().expect(".vals scalar");
            let extent = scalar.header().data_type.extent as usize;
            println!("\n.vals extent: {} doubles", extent);
            let byte_count = extent * 8;
            let mut buf = vec![0u8; byte_count];
            scalar.read_sample(0, &mut buf).expect("read .vals");
            let doubles: &[f64] = bytemuck::try_cast_slice(&buf).unwrap_or(&[]);
            println!(".vals values:");
            for (i, chunk) in doubles.chunks(3).enumerate() {
                println!("  op[{}]: ({:.4}, {:.4}, {:.4})", i, chunk[0], chunk[1], chunk[2]);
            }
        }
    }
    
    // Read .inherits
    {
        let inh_prop = xform.property_by_name(".inherits").expect(".inherits");
        let scalar = inh_prop.as_scalar().expect(".inherits scalar");
        let mut buf = [0u8; 1];
        scalar.read_sample(0, &mut buf).expect("read .inherits");
        println!("\n.inherits: {}", buf[0] != 0);
    }
    
    // Also check what our IXform produces
    println!("\n=== IXform interpretation ===");
    if let Some(ixform) = alembic::geom::IXform::new(&brake_disc) {
        let sample = ixform.get_sample(0).expect("get_sample");
        println!("Inherits: {}", sample.inherits);
        println!("Ops count: {}", sample.ops.len());
        for (i, op) in sample.ops.iter().enumerate() {
            println!("  op[{}]: {:?} values={:?}", i, op.op_type, op.values);
        }
        let matrix = sample.matrix();
        println!("Computed matrix:");
        let cols = matrix.to_cols_array_2d();
        println!("  [{:.4}, {:.4}, {:.4}, {:.4}]", cols[0][0], cols[1][0], cols[2][0], cols[3][0]);
        println!("  [{:.4}, {:.4}, {:.4}, {:.4}]", cols[0][1], cols[1][1], cols[2][1], cols[3][1]);
        println!("  [{:.4}, {:.4}, {:.4}, {:.4}]", cols[0][2], cols[1][2], cols[2][2], cols[3][2]);
        println!("  [{:.4}, {:.4}, {:.4}, {:.4}]", cols[0][3], cols[1][3], cols[2][3], cols[3][3]);
    }
}
