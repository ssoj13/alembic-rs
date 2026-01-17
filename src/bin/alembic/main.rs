//! Alembic CLI - Tool for inspecting and manipulating Alembic files.

use alembic::prelude::{IObject, IPolyMesh, ISubD, ICurves, IPoints, ICamera, IXform, INuPatch, ILight, IFaceSet};
use alembic::abc::ICompoundProperty;
use alembic::abc::IArchive as AbcIArchive;
use alembic::ogawa::writer::{
    OArchive, OObject, OPolyMesh, OPolyMeshSample, OXform, OXformSample,
    OSubD, OSubDSample, OCurves, OCurvesSample, OPoints, OPointsSample,
    OCamera, ONuPatch, ONuPatchSample, OLight, OFaceSet, OFaceSetSample,
};
use std::env;
use std::path::Path;

use std::sync::atomic::{AtomicU8, Ordering};

/// Verbosity level (thread-safe)
const LOG_QUIET: u8 = 0;
const LOG_INFO: u8 = 1;
const LOG_DEBUG: u8 = 2;
const LOG_TRACE: u8 = 3;

static LOG_LEVEL: AtomicU8 = AtomicU8::new(LOG_INFO);

#[inline]
fn log_level() -> u8 {
    LOG_LEVEL.load(Ordering::Relaxed)
}

#[inline]
fn set_log_level(level: u8) {
    LOG_LEVEL.store(level, Ordering::Relaxed);
}

macro_rules! info {
    ($($arg:tt)*) => {
        if log_level() >= LOG_INFO {
            println!("[INFO] {}", format!($($arg)*));
        }
    };
}

macro_rules! debug {
    ($($arg:tt)*) => {
        if log_level() >= LOG_DEBUG {
            println!("[DEBUG] {}", format!($($arg)*));
        }
    };
}

macro_rules! trace {
    ($($arg:tt)*) => {
        if log_level() >= LOG_TRACE {
            println!("[TRACE] {}", format!($($arg)*));
        }
    };
}

fn main() {
    let args: Vec<String> = env::args().collect();
    
    // Parse global flags
    let mut filtered_args: Vec<&str> = Vec::new();
    for arg in &args[1..] {
        match arg.as_str() {
            "-v" | "--verbose" => set_log_level(LOG_DEBUG),
            "-vv" | "--trace" => set_log_level(LOG_TRACE),
            "-q" | "--quiet" => set_log_level(LOG_QUIET),
            _ => filtered_args.push(arg),
        }
    }
    
    if filtered_args.is_empty() {
        print_help();
        return;
    }
    
    match filtered_args[0] {
        // View command - launch 3D viewer
        "view" | "v" => {
            #[cfg(feature = "viewer")]
            {
                let file = filtered_args.get(1).map(|s| std::path::PathBuf::from(*s));
                if let Err(e) = alembic::viewer::run(file) {
                    eprintln!("Viewer error: {}", e);
                    std::process::exit(1);
                }
            }
            #[cfg(not(feature = "viewer"))]
            {
                eprintln!("Viewer not available. Rebuild with: cargo build --features viewer");
                std::process::exit(1);
            }
        }
        
        // Info command - show archive summary
        "info" | "i" => {
            if filtered_args.len() < 2 {
                eprintln!("Error: missing file argument");
                eprintln!("Usage: alembic info <file.abc>");
                std::process::exit(1);
            }
            cmd_info(filtered_args[1]);
        }
        
        // Tree command - show hierarchy
        "tree" | "t" => {
            if filtered_args.len() < 2 {
                eprintln!("Error: missing file argument");
                eprintln!("Usage: alembic tree <file.abc>");
                std::process::exit(1);
            }
            cmd_tree(filtered_args[1]);
        }
        
        // Stats command - detailed statistics
        "stats" | "s" => {
            if filtered_args.len() < 2 {
                eprintln!("Error: missing file argument");
                eprintln!("Usage: alembic stats <file.abc>");
                std::process::exit(1);
            }
            cmd_stats(filtered_args[1]);
        }
        
        // Dump command - xform details
        "dump" | "d" => {
            if filtered_args.len() < 2 {
                eprintln!("Error: missing file argument");
                eprintln!("Usage: alembic dump <file.abc> [pattern] [--json]");
                std::process::exit(1);
            }
            let json_mode = filtered_args.iter().any(|&s| s == "--json" || s == "-j");
            if json_mode {
                set_log_level(LOG_QUIET);
            }
            let pattern = filtered_args.get(2).filter(|&&s| s != "--json" && s != "-j").copied();
            cmd_dump(filtered_args[1], pattern, json_mode);
        }
        
        // Meta command - show object/property metadata
        "meta" | "m" => {
            if filtered_args.len() < 2 {
                eprintln!("Error: missing file argument");
                eprintln!("Usage: alembic meta <file.abc> [object_pattern]");
                std::process::exit(1);
            }
            let pattern = filtered_args.get(2).copied();
            cmd_meta(filtered_args[1], pattern);
        }
        
        // Copy command - round-trip test
        "copy" | "c" => {
            if filtered_args.len() < 3 {
                eprintln!("Error: missing arguments");
                eprintln!("Usage: alembic copy <input.abc> <output.abc>");
                std::process::exit(1);
            }
            cmd_copy(filtered_args[1], filtered_args[2]);
        }
        
        // Copy2 command - full re-write using our writer (ALL schema types)
        "copy2" | "c2" => {
            if filtered_args.len() < 3 {
                eprintln!("Error: missing arguments");
                eprintln!("Usage: alembic copy2 <input.abc> <output.abc>");
                std::process::exit(1);
            }
            cmd_copy2(filtered_args[1], filtered_args[2]);
        }
        
        // Help
        "help" | "h" | "-h" | "--help" => print_help(),
        
        // Default: if file exists, show info; otherwise error
        _ => {
            if Path::new(filtered_args[0]).exists() {
                cmd_info(filtered_args[0]);
            } else {
                eprintln!("Unknown command: {}", filtered_args[0]);
                eprintln!();
                print_help();
                std::process::exit(1);
            }
        }
    }
}

fn print_help() {
    println!("alembic - Alembic file toolkit");
    println!();
    println!("USAGE:");
    println!("    alembic [OPTIONS] <COMMAND> [ARGS]");
    println!();
    println!("COMMANDS:");
    println!("    v, view   <file>              Open file in 3D viewer (Esc to exit)");
    println!("    i, info   <file>              Show archive info and object counts");
    println!("    t, tree   <file>              Show full object hierarchy");
    println!("    s, stats  <file>              Show detailed statistics with timing info");
    println!("    d, dump   <file> [pattern]    Dump xform transforms (filter by pattern)");
    println!("    m, meta   <file> [pattern]    Show object/property metadata");
    println!("    c, copy   <in> <out>          Copy archive (Xform + PolyMesh only)");
    println!("    c2, copy2 <in> <out>          Full re-write using our writer (ALL types)");
    println!("    h, help                       Show this help");
    println!();
    println!("OPTIONS:");
    println!("    -v, --verbose    Show debug output");
    println!("    -vv, --trace     Show trace output (very verbose)");
    println!("    -q, --quiet      Suppress all output");
    println!();
    println!("EXAMPLES:");
    println!("    alembic view model.abc                # Open in 3D viewer");
    println!("    alembic info scene.abc                # Quick overview");
    println!("    alembic tree character.abc            # See hierarchy");
    println!("    alembic dump scene.abc wheel          # Dump transforms matching 'wheel'");
    println!("    alembic dump scene.abc --json         # Export all transforms as JSON");
    println!("    alembic copy input.abc output.abc     # Test round-trip");
    println!("    alembic -v info large.abc             # Verbose info");
    println!();
    println!("NOTES:");
    println!("    - Passing a .abc file directly is equivalent to 'info'");
    println!("    - Viewer requires --features viewer (enabled by default)");
    println!("    - Press Esc to close the viewer");
}

fn cmd_info(path: &str) {
    info!("Opening archive: {}", path);
    
    let archive = match AbcIArchive::open(path) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("Failed to open {}: {}", path, e);
            std::process::exit(1);
        }
    };
    
    debug!("Archive opened successfully");
    
    println!("Archive: {}", path);
    println!("Version: {}", archive.getArchiveVersion());
    println!("Time samplings: {}", archive.getNumTimeSamplings());
    println!();
    
    // Count objects by type
    let root = archive.getTop();
    trace!("Starting object count from root");
    
    let mut counts = ObjectCounts::default();
    count_objects(&root, &mut counts);
    
    debug!("Counted {} total objects", counts.total());
    
    println!("Objects:");
    println!("  Xforms:  {}", counts.xform);
    println!("  Meshes:  {} ({} vertices, {} faces)", counts.mesh, counts.total_verts, counts.total_faces);
    println!("  SubDs:   {}", counts.subd);
    println!("  Curves:  {}", counts.curve);
    println!("  Points:  {}", counts.point);
    println!("  Cameras: {}", counts.camera);
    println!("  Lights:  {}", counts.light);
    if counts.other > 0 {
        println!("  Other:   {}", counts.other);
    }
    println!();
    println!("Total objects: {}", counts.total());
}

fn cmd_tree(path: &str) {
    info!("Opening archive: {}", path);
    
    let archive = match AbcIArchive::open(path) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("Failed to open {}: {}", path, e);
            std::process::exit(1);
        }
    };
    
    println!("Archive: {}", path);
    println!();
    
    let root = archive.getTop();
    print_tree(&root, 0);
}

fn cmd_stats(path: &str) {
    info!("Opening archive: {}", path);
    
    let archive = match AbcIArchive::open(path) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("Failed to open {}: {}", path, e);
            std::process::exit(1);
        }
    };
    
    println!("Archive: {}", path);
    println!("Version: {}", archive.getArchiveVersion());
    println!();
    
    // Time samplings
    println!("Time Samplings ({}):", archive.getNumTimeSamplings());
    for i in 0..archive.getNumTimeSamplings() {
        if let Some(ts) = archive.getTimeSampling(i) {
            let type_str = if ts.is_identity() {
                "Identity".to_string()
            } else if ts.is_uniform() {
                format!("Uniform ({}fps)", (1.0_f64 / ts.time_per_cycle()).round())
            } else if ts.is_cyclic() {
                format!("Cyclic ({} per cycle)", ts.samples_per_cycle())
            } else {
                format!("Acyclic ({} times)", ts.num_stored_times())
            };
            
            let max_samples = archive.getMaxNumSamplesForTimeSamplingIndex(i).unwrap_or(0);
            println!("  [{}] {} - {} samples", i, type_str, max_samples);
        }
    }
    println!();
    
    // Object stats
    let root = archive.getTop();
    println!("Object Hierarchy:");
    print_stats_tree(&root, 0);
}

/// Object counts for statistics
#[derive(Default)]
struct ObjectCounts {
    xform: usize,
    mesh: usize,
    subd: usize,
    curve: usize,
    point: usize,
    camera: usize,
    light: usize,
    other: usize,
    total_verts: usize,
    total_faces: usize,
}

impl ObjectCounts {
    fn total(&self) -> usize {
        self.xform + self.mesh + self.subd + self.curve + 
        self.point + self.camera + self.light + self.other
    }
}

fn count_objects(obj: &IObject, counts: &mut ObjectCounts) {
    let schema = obj.getMetaData().get("schema").unwrap_or_default();
    let schema_str = schema;
    trace!("Processing object: {} [{}]", obj.getName(), schema);
    
    if schema_str.contains("Xform") {
        counts.xform += 1;
    } else if schema_str.contains("PolyMesh") {
        counts.mesh += 1;
        if let Some(poly) = IPolyMesh::new(obj) {
            if let Ok(sample) = poly.getSample(0) {
                counts.total_verts += sample.positions.len();
                counts.total_faces += sample.face_counts.len();
            }
        }
    } else if schema_str.contains("SubD") {
        counts.subd += 1;
        if let Some(sd) = ISubD::new(obj) {
            if let Ok(sample) = sd.getSample(0) {
                counts.total_verts += sample.positions.len();
                counts.total_faces += sample.face_counts.len();
            }
        }
    } else if schema_str.contains("Curve") {
        counts.curve += 1;
    } else if schema_str.contains("Points") {
        counts.point += 1;
    } else if schema_str.contains("Camera") {
        counts.camera += 1;
    } else if schema_str.contains("Light") {
        counts.light += 1;
    } else if !schema_str.is_empty() {
        counts.other += 1;
    }
    
    for i in 0..obj.getNumChildren() {
        if let Some(child) = obj.getChild(i) {
            count_objects(&child, counts);
        }
    }
}

fn print_tree(obj: &IObject, depth: usize) {
    let indent = "  ".repeat(depth);
    let schema = obj.getMetaData().get("schema").unwrap_or_default();
    let type_str = schema_to_type(schema);
    
    if depth == 0 {
        println!("{}/", obj.getName());
    } else {
        println!("{}{} [{}]", indent, obj.getName(), type_str);
    }
    
    for i in 0..obj.getNumChildren() {
        if let Some(child) = obj.getChild(i) {
            print_tree(&child, depth + 1);
        }
    }
}

fn print_stats_tree(obj: &IObject, depth: usize) {
    let indent = "  ".repeat(depth);
    let schema = obj.getMetaData().get("schema").unwrap_or_default();
    let type_str = schema_to_type(schema);
    
    // Get additional info based on type
    let extra_info = get_object_info(obj, schema);
    
    if depth == 0 {
        println!("{}/", obj.getName());
    } else if extra_info.is_empty() {
        println!("{}{} [{}]", indent, obj.getName(), type_str);
    } else {
        println!("{}{} [{}] {}", indent, obj.getName(), type_str, extra_info);
    }
    
    for i in 0..obj.getNumChildren() {
        if let Some(child) = obj.getChild(i) {
            print_stats_tree(&child, depth + 1);
        }
    }
}

fn schema_to_type(schema: &str) -> &str {
    if schema.contains("Xform") { "Xform" }
    else if schema.contains("PolyMesh") { "PolyMesh" }
    else if schema.contains("SubD") { "SubD" }
    else if schema.contains("Curves") { "Curves" }
    else if schema.contains("Points") { "Points" }
    else if schema.contains("Camera") { "Camera" }
    else if schema.contains("Light") { "Light" }
    else if schema.contains("FaceSet") { "FaceSet" }
    else if schema.contains("NuPatch") { "NuPatch" }
    else if schema.contains("Material") { "Material" }
    else if schema.is_empty() { "Group" }
    else { schema }
}

fn cmd_meta(path: &str, pattern: Option<&str>) {
    info!("Opening archive: {}", path);
    
    let archive = match AbcIArchive::open(path) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("Failed to open {}: {}", path, e);
            std::process::exit(1);
        }
    };
    
    println!("Archive: {}", path);
    println!("Version: {}", archive.getArchiveVersion());
    
    // Archive metadata
    let meta = archive.getArchiveMetaData();
    println!("\nArchive Metadata:");
    for (k, v) in meta.iter() {
        println!("  {}: {}", k, v);
    }
    
    // Time samplings
    println!("\nTime Samplings: {}", archive.getNumTimeSamplings());
    for i in 0..archive.getNumTimeSamplings() {
        if let Some(ts) = archive.getTimeSampling(i) {
            println!("  [{}] {:?}", i, ts);
        }
    }
    
    println!("\nObject Metadata{}", if let Some(p) = pattern { format!(" (filter: {})", p) } else { String::new() });
    let root = archive.getTop();
    dump_object_meta(&root, 0, pattern);
}

fn dump_object_meta(obj: &IObject, depth: usize, pattern: Option<&str>) {
    let indent = "  ".repeat(depth);
    let name = obj.getName();
    let full_name = obj.getFullName();
    
    let matches = pattern.map(|p| full_name.contains(p) || name.contains(p)).unwrap_or(true);
    
    if matches {
        let meta = obj.getMetaData();
        let schema = meta.get("schema").unwrap_or_default();
        let type_str = schema_to_type(&schema);
        
        println!("{}[{}] {}", indent, type_str, name);
        
        // Print object metadata
        if !meta.is_empty() {
            println!("{}  metadata:", indent);
            for (k, v) in meta.iter() {
                println!("{}    {}: {}", indent, k, v);
            }
        }
        
        // Print properties
        let props = obj.getProperties();
        let num_props = props.getNumProperties();
        if num_props > 0 {
            println!("{}  properties: {}", indent, num_props);
            dump_properties(&props, depth + 2);
        }
        println!();
    }
    
    for child in obj.getChildren() {
        dump_object_meta(&child, depth + 1, pattern);
    }
}

fn dump_properties(props: &ICompoundProperty, depth: usize) {
    let indent = "  ".repeat(depth);
    
    for i in 0..props.getNumProperties() {
        if let Some(header) = props.getPropertyHeader(i) {
            let prop_type = if header.is_compound() { "compound" }
                else if header.is_array() { "array" }
                else { "scalar" };
            
            let dtype = format!("{:?}", header.data_type);
            println!("{}[{}] {} ({})", indent, prop_type, header.name, dtype);
            
            // Print property metadata if any
            if !header.meta_data.is_empty() {
                for (k, v) in header.meta_data.iter() {
                    println!("{}  {}: {}", indent, k, v);
                }
            }
            
            // Recurse into compound
            if header.is_compound() {
                if let Some(prop) = props.getPropertyByName(&header.name) {
                    if let Some(child) = prop.asCompound() {
                        dump_properties(&child, depth + 1);
                    }
                }
            }
        }
    }
}

fn cmd_dump(path: &str, pattern: Option<&str>, json_mode: bool) {
    info!("Opening archive: {}", path);
    
    let archive = match AbcIArchive::open(path) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("Failed to open {}: {}", path, e);
            std::process::exit(1);
        }
    };
    
    if json_mode {
        let root = archive.getTop();
        let mut objects = Vec::new();
        collect_dump_json(&root, glam::Mat4::IDENTITY, pattern, &mut objects);
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
            "archive": path,
            "objects": objects
        })).unwrap_or_default());
    } else {
        println!("Archive: {}", path);
        println!("Xform Dump{}", if let Some(p) = pattern { format!(" (filter: {})", p) } else { String::new() });
        println!();
        let root = archive.getTop();
        dump_xforms(&root, 0, glam::Mat4::IDENTITY, pattern);
    }
}

fn dump_xforms(obj: &IObject, depth: usize, parent_world: glam::Mat4, pattern: Option<&str>) {
    let indent = "  ".repeat(depth);
    let name = obj.getName();
    let full_name = obj.getFullName();
    
    // Check pattern filter
    let matches_pattern = pattern.map(|p| full_name.contains(p) || name.contains(p)).unwrap_or(true);
    
    // Get local transform
    let local_matrix = if let Some(xform) = IXform::new(obj) {
        if let Ok(sample) = xform.getSample(0) {
            let m = sample.matrix();
            let inherits = sample.inherits;
            
            if matches_pattern {
                println!("{}[XFORM] {} (inherits={})", indent, name, inherits);
                println!("{}  ops: {}", indent, sample.ops.len());
                for (i, op) in sample.ops.iter().enumerate() {
                    println!("{}    [{i}] {:?}: {:?}", indent, op.op_type, op.values);
                }
                println!("{}  local matrix:", indent);
                print_matrix(&m, &indent);
                
                let world = if inherits { parent_world * m } else { m };
                println!("{}  world matrix:", indent);
                print_matrix(&world, &indent);
                println!();
            }
            
            if sample.inherits { parent_world * m } else { m }
        } else {
            parent_world
        }
    } else {
        // Not an xform, check if it's a mesh
        let schema = obj.getMetaData().get("schema").unwrap_or_default();
        if matches_pattern && (schema.contains("PolyMesh") || schema.contains("SubD")) {
            println!("{}[MESH] {} (world from parent)", indent, name);
            println!("{}  world matrix:", indent);
            print_matrix(&parent_world, &indent);
            
            // Extract TRS from world matrix for readability
            let (scale, rotation, translation) = parent_world.to_scale_rotation_translation();
            let euler = rotation.to_euler(glam::EulerRot::XYZ);
            println!("{}  decomposed:", indent);
            println!("{}    T: ({:.4}, {:.4}, {:.4})", indent, translation.x, translation.y, translation.z);
            println!("{}    R: ({:.2}, {:.2}, {:.2}) deg", indent, 
                euler.0.to_degrees(), euler.1.to_degrees(), euler.2.to_degrees());
            println!("{}    S: ({:.4}, {:.4}, {:.4})", indent, scale.x, scale.y, scale.z);
            println!();
        }
        parent_world
    };
    
    // Recurse into children
    for child in obj.getChildren() {
        dump_xforms(&child, depth + 1, local_matrix, pattern);
    }
}

fn print_matrix(m: &glam::Mat4, indent: &str) {
    let cols = m.to_cols_array_2d();
    // Print as rows (transposed view for readability)
    println!("{}    [{:>10.4} {:>10.4} {:>10.4} {:>10.4}]", indent, cols[0][0], cols[1][0], cols[2][0], cols[3][0]);
    println!("{}    [{:>10.4} {:>10.4} {:>10.4} {:>10.4}]", indent, cols[0][1], cols[1][1], cols[2][1], cols[3][1]);
    println!("{}    [{:>10.4} {:>10.4} {:>10.4} {:>10.4}]", indent, cols[0][2], cols[1][2], cols[2][2], cols[3][2]);
    println!("{}    [{:>10.4} {:>10.4} {:>10.4} {:>10.4}]", indent, cols[0][3], cols[1][3], cols[2][3], cols[3][3]);
}

fn collect_dump_json(
    obj: &IObject,
    parent_world: glam::Mat4,
    pattern: Option<&str>,
    out: &mut Vec<serde_json::Value>,
) -> glam::Mat4 {
    let name = obj.getName();
    let full_name = obj.getFullName();
    let matches = pattern.map(|p| full_name.contains(p) || name.contains(p)).unwrap_or(true);
    
    let local_matrix = if let Some(xform) = IXform::new(obj) {
        if let Ok(sample) = xform.getSample(0) {
            let m = sample.matrix();
            let world = if sample.inherits { parent_world * m } else { m };
            
            if matches {
                let ops: Vec<serde_json::Value> = sample.ops.iter().map(|op| {
                    serde_json::json!({
                        "type": format!("{:?}", op.op_type),
                        "values": op.values
                    })
                }).collect();
                
                out.push(serde_json::json!({
                    "type": "xform",
                    "name": name,
                    "path": full_name,
                    "inherits": sample.inherits,
                    "ops": ops,
                    "local": mat4_to_array(&m),
                    "world": mat4_to_array(&world)
                }));
            }
            world
        } else {
            parent_world
        }
    } else {
        let schema = obj.getMetaData().get("schema").unwrap_or_default();
        if matches && (schema.contains("PolyMesh") || schema.contains("SubD")) {
            let (scale, rot, trans) = parent_world.to_scale_rotation_translation();
            let euler = rot.to_euler(glam::EulerRot::XYZ);
            out.push(serde_json::json!({
                "type": "mesh",
                "name": name,
                "path": full_name,
                "world": mat4_to_array(&parent_world),
                "decomposed": {
                    "translate": [trans.x, trans.y, trans.z],
                    "rotate_deg": [euler.0.to_degrees(), euler.1.to_degrees(), euler.2.to_degrees()],
                    "scale": [scale.x, scale.y, scale.z]
                }
            }));
        }
        parent_world
    };
    
    for child in obj.getChildren() {
        collect_dump_json(&child, local_matrix, pattern, out);
    }
    local_matrix
}

fn mat4_to_array(m: &glam::Mat4) -> [[f32; 4]; 4] {
    m.to_cols_array_2d()
}

fn cmd_copy(input: &str, output: &str) {
    info!("Copying {} -> {}", input, output);
    
    let archive = match AbcIArchive::open(input) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("Failed to open {}: {}", input, e);
            std::process::exit(1);
        }
    };
    
    let mut out_archive = match OArchive::create(output) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("Failed to create {}: {}", output, e);
            std::process::exit(1);
        }
    };
    
    let root = archive.getTop();
    let mut out_root = OObject::new("");
    
    // Copy children recursively
    for child in root.getChildren() {
        if let Some(out_child) = copy_object(&child) {
            out_root.add_child(out_child);
        }
    }
    
    if let Err(e) = out_archive.write_archive(&out_root) {
        eprintln!("Failed to write archive: {}", e);
        std::process::exit(1);
    }
    
    println!("Copied {} -> {}", input, output);
}

fn copy_object(obj: &IObject) -> Option<OObject> {
    let name = obj.getName();
    let schema = obj.getMetaData().get("schema").unwrap_or_default();
    
    debug!("Copying object: {} [{}]", name, schema);
    
    // Handle different schema types
    if schema.contains("Xform") {
        if let Some(xform) = IXform::new(obj) {
            let mut out_xform = OXform::new(name);
            
            // Copy all samples - convert ops to matrix
            for i in 0..xform.getNumSamples() {
                if let Ok(sample) = xform.getSample(i) {
                    let matrix = sample.matrix();
                    out_xform.add_sample(OXformSample::from_matrix(matrix, sample.inherits));
                }
            }
            
            let mut out_obj = out_xform.build();
            
            // Copy children
            for child in obj.getChildren() {
                if let Some(out_child) = copy_object(&child) {
                    out_obj.add_child(out_child);
                }
            }
            
            return Some(out_obj);
        }
    } else if schema.contains("PolyMesh") {
        if let Some(mesh) = IPolyMesh::new(obj) {
            let mut out_mesh = OPolyMesh::new(name);
            
            // Copy all samples
            for i in 0..mesh.getNumSamples() {
                if let Ok(sample) = mesh.getSample(i) {
                    let out_sample = OPolyMeshSample::new(
                        sample.positions.clone(),
                        sample.face_counts.clone(),
                        sample.face_indices.clone(),
                    );
                    out_mesh.add_sample(&out_sample);
                }
            }
            
            return Some(out_mesh.build());
        }
    }
    
    // Generic object - just copy children
    let mut out_obj = OObject::new(name);
    for child in obj.getChildren() {
        if let Some(out_child) = copy_object(&child) {
            out_obj.add_child(out_child);
        }
    }
    Some(out_obj)
}

fn get_object_info(obj: &IObject, schema: &str) -> String {
    if schema.contains("PolyMesh") {
        if let Some(poly) = IPolyMesh::new(obj) {
            let samples = poly.getNumSamples();
            if let Ok(sample) = poly.getSample(0) {
                return format!("- {} verts, {} faces, {} samples", 
                    sample.positions.len(), sample.face_counts.len(), samples);
            }
        }
    } else if schema.contains("SubD") {
        if let Some(sd) = ISubD::new(obj) {
            let samples = sd.getNumSamples();
            if let Ok(sample) = sd.getSample(0) {
                return format!("- {} verts, {} faces, {} samples",
                    sample.positions.len(), sample.face_counts.len(), samples);
            }
        }
    } else if schema.contains("Curves") {
        if let Some(curves) = ICurves::new(obj) {
            let samples = curves.getNumSamples();
            if let Ok(sample) = curves.getSample(0) {
                return format!("- {} curves, {} samples",
                    sample.num_vertices.len(), samples);
            }
        }
    } else if schema.contains("Points") {
        if let Some(points) = IPoints::new(obj) {
            let samples = points.getNumSamples();
            if let Ok(sample) = points.getSample(0) {
                return format!("- {} points, {} samples",
                    sample.positions.len(), samples);
            }
        }
    } else if schema.contains("Camera") {
        if let Some(cam) = ICamera::new(obj) {
            let samples = cam.getNumSamples();
            if let Ok(sample) = cam.getSample(0) {
                return format!("- focal={:.1}mm, {} samples",
                    sample.focal_length, samples);
            }
        }
    } else if schema.contains("Xform") {
        if let Some(xf) = IXform::new(obj) {
            let samples = xf.getNumSamples();
            let constant = if xf.isConstant() { "static" } else { "animated" };
            return format!("- {}, {} samples", constant, samples);
        }
    }
    String::new()
}

// ============================================================================
// copy2 - Full re-write using our writer (ALL schema types)
// ============================================================================

fn cmd_copy2(input: &str, output: &str) {
    info!("Full re-write {} -> {} (ALL schema types)", input, output);
    
    let archive = match AbcIArchive::open(input) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("Failed to open {}: {}", input, e);
            std::process::exit(1);
        }
    };
    
    let mut out_archive = match OArchive::create(output) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("Failed to create {}: {}", output, e);
            std::process::exit(1);
        }
    };
    
    // Copy archive metadata from source file
    out_archive.set_archive_metadata(archive.getArchiveMetaData().clone());
    
    // Copy time samplings from input archive
    // Skip index 0 (identity time sampling - always present)
    for i in 1..archive.getNumTimeSamplings() {
        if let Some(ts) = archive.getTimeSampling(i) {
            out_archive.addTimeSampling(ts.clone());
        }
    }
    
    let root = archive.getTop();
    let mut out_root = OObject::new("");
    
    let mut stats = CopyStats::default();
    
    // Copy children recursively
    for child in root.getChildren() {
        if let Some(out_child) = copy2_object(&child, &archive, &mut stats) {
            out_root.add_child(out_child);
        }
    }
    
    if let Err(e) = out_archive.write_archive(&out_root) {
        eprintln!("Failed to write archive: {}", e);
        std::process::exit(1);
    }
    
    println!("Full re-write {} -> {}", input, output);
    println!("  Xforms:   {}", stats.xform);
    println!("  PolyMesh: {}", stats.polymesh);
    println!("  SubD:     {}", stats.subd);
    println!("  Curves:   {}", stats.curves);
    println!("  Points:   {}", stats.points);
    println!("  Camera:   {}", stats.camera);
    println!("  NuPatch:  {}", stats.nupatch);
    println!("  Light:    {}", stats.light);
    println!("  FaceSet:  {}", stats.faceset);
    println!("  Other:    {}", stats.other);
    println!("  Total:    {}", stats.total());
}

#[derive(Default)]
struct CopyStats {
    xform: usize,
    polymesh: usize,
    subd: usize,
    curves: usize,
    points: usize,
    camera: usize,
    nupatch: usize,
    light: usize,
    faceset: usize,
    other: usize,
}

impl CopyStats {
    fn total(&self) -> usize {
        self.xform + self.polymesh + self.subd + self.curves + 
        self.points + self.camera + self.nupatch + self.light + 
        self.faceset + self.other
    }
}

fn copy2_object(obj: &IObject, archive: &AbcIArchive, stats: &mut CopyStats) -> Option<OObject> {
    let name = obj.getName();
    let schema = obj.getMetaData().get("schema").unwrap_or_default();
    
    debug!("copy2: {} [{}]", name, schema);
    
    // Handle different schema types
    if schema.contains("Xform") {
        return copy2_xform(obj, archive, stats);
    } else if schema.contains("PolyMesh") {
        return copy2_polymesh(obj, archive, stats);
    } else if schema.contains("SubD") {
        return copy2_subd(obj, archive, stats);
    } else if schema.contains("Curve") {
        return copy2_curves(obj, archive, stats);
    } else if schema.contains("Points") {
        return copy2_points(obj, archive, stats);
    } else if schema.contains("Camera") {
        return copy2_camera(obj, archive, stats);
    } else if schema.contains("NuPatch") {
        return copy2_nupatch(obj, archive, stats);
    } else if schema.contains("Light") {
        return copy2_light(obj, archive, stats);
    } else if schema.contains("FaceSet") {
        return copy2_faceset(obj, archive, stats);
    }
    
    // Generic object - just copy children
    stats.other += 1;
    let mut out_obj = OObject::new(name);
    for child in obj.getChildren() {
        if let Some(out_child) = copy2_object(&child, archive, stats) {
            out_obj.add_child(out_child);
        }
    }
    Some(out_obj)
}

fn copy2_xform(obj: &IObject, archive: &AbcIArchive, stats: &mut CopyStats) -> Option<OObject> {
    let name = obj.getName();
    if let Some(xform) = IXform::new(obj) {
        stats.xform += 1;
        let mut out_xform = OXform::new(name);
        
        // Copy time sampling
        out_xform.set_time_sampling(xform.getTimeSamplingIndex());
        
        for i in 0..xform.getNumSamples() {
            if let Ok(sample) = xform.getSample(i) {
                let matrix = sample.matrix();
                out_xform.add_sample(OXformSample::from_matrix(matrix, sample.inherits));
            }
        }
        
        let mut out_obj = out_xform.build();
        for child in obj.getChildren() {
            if let Some(out_child) = copy2_object(&child, archive, stats) {
                out_obj.add_child(out_child);
            }
        }
        return Some(out_obj);
    }
    // Schema check passed but IXform::new failed - fallback to generic copy with children
    debug!("copy2_xform: IXform::new failed for {}, using generic copy", name);
    let mut out_obj = OObject::new(name);
    for child in obj.getChildren() {
        if let Some(out_child) = copy2_object(&child, archive, stats) {
            out_obj.add_child(out_child);
        }
    }
    Some(out_obj)
}

fn copy2_polymesh(obj: &IObject, archive: &AbcIArchive, stats: &mut CopyStats) -> Option<OObject> {
    let name = obj.getName();
    if let Some(mesh) = IPolyMesh::new(obj) {
        stats.polymesh += 1;
        let mut out_mesh = OPolyMesh::new(name);
        
        // Copy time sampling
        out_mesh.set_time_sampling(mesh.getTimeSamplingIndex());
        
        for i in 0..mesh.getNumSamples() {
            if let Ok(sample) = mesh.getSample(i) {
                let mut out_sample = OPolyMeshSample::new(
                    sample.positions.clone(),
                    sample.face_counts.clone(),
                    sample.face_indices.clone(),
                );
                out_sample.velocities = sample.velocities.clone();
                out_sample.normals = sample.normals.clone();
                out_mesh.add_sample(&out_sample);
            }
        }
        
        let mut out_obj = out_mesh.build();
        // Copy child FaceSets if any
        for child in obj.getChildren() {
            if let Some(out_child) = copy2_object(&child, archive, stats) {
                out_obj.add_child(out_child);
            }
        }
        return Some(out_obj);
    }
    // Fallback
    debug!("copy2_polymesh: IPolyMesh::new failed for {}", name);
    let mut out_obj = OObject::new(name);
    for child in obj.getChildren() {
        if let Some(out_child) = copy2_object(&child, archive, stats) {
            out_obj.add_child(out_child);
        }
    }
    Some(out_obj)
}

fn copy2_subd(obj: &IObject, archive: &AbcIArchive, stats: &mut CopyStats) -> Option<OObject> {
    let name = obj.getName();
    if let Some(sd) = ISubD::new(obj) {
        stats.subd += 1;
        let mut out_sd = OSubD::new(name);
        
        // Copy time sampling
        out_sd.set_time_sampling(sd.getTimeSamplingIndex());
        
        for i in 0..sd.getNumSamples() {
            if let Ok(sample) = sd.getSample(i) {
                let mut out_sample = OSubDSample::new(
                    sample.positions.clone(),
                    sample.face_counts.clone(),
                    sample.face_indices.clone(),
                );
                // Copy optional fields (velocities already Option, others are plain Vec)
                out_sample.velocities = sample.velocities.clone();
                if !sample.crease_indices.is_empty() {
                    out_sample.crease_indices = Some(sample.crease_indices.clone());
                }
                if !sample.crease_lengths.is_empty() {
                    out_sample.crease_lengths = Some(sample.crease_lengths.clone());
                }
                if !sample.crease_sharpnesses.is_empty() {
                    out_sample.crease_sharpnesses = Some(sample.crease_sharpnesses.clone());
                }
                if !sample.corner_indices.is_empty() {
                    out_sample.corner_indices = Some(sample.corner_indices.clone());
                }
                if !sample.corner_sharpnesses.is_empty() {
                    out_sample.corner_sharpnesses = Some(sample.corner_sharpnesses.clone());
                }
                if !sample.holes.is_empty() {
                    out_sample.holes = Some(sample.holes.clone());
                }
                out_sd.add_sample(&out_sample);
            }
        }
        
        let mut out_obj = out_sd.build();
        for child in obj.getChildren() {
            if let Some(out_child) = copy2_object(&child, archive, stats) {
                out_obj.add_child(out_child);
            }
        }
        return Some(out_obj);
    }
    None
}

fn copy2_curves(obj: &IObject, _archive: &AbcIArchive, stats: &mut CopyStats) -> Option<OObject> {
    let name = obj.getName();
    if let Some(curves) = ICurves::new(obj) {
        stats.curves += 1;
        let mut out_curves = OCurves::new(name);
        
        // Copy time sampling
        out_curves.set_time_sampling(curves.getTimeSamplingIndex());
        
        for i in 0..curves.getNumSamples() {
            if let Ok(sample) = curves.getSample(i) {
                let mut out_sample = OCurvesSample::new(
                    sample.positions.clone(),
                    sample.num_vertices.clone(),
                );
                out_sample.curve_type = sample.curve_type;
                out_sample.wrap = sample.wrap;
                out_sample.basis = sample.basis;
                // velocities is Option, others are plain Vec
                out_sample.velocities = sample.velocities.clone();
                if !sample.widths.is_empty() {
                    out_sample.widths = Some(sample.widths.clone());
                }
                if !sample.normals.is_empty() {
                    out_sample.normals = Some(sample.normals.clone());
                }
                if !sample.uvs.is_empty() {
                    out_sample.uvs = Some(sample.uvs.clone());
                }
                out_curves.add_sample(&out_sample);
            }
        }
        
        return Some(out_curves.build());
    }
    None
}

fn copy2_points(obj: &IObject, _archive: &AbcIArchive, stats: &mut CopyStats) -> Option<OObject> {
    let name = obj.getName();
    if let Some(points) = IPoints::new(obj) {
        stats.points += 1;
        let mut out_points = OPoints::new(name);
        
        // Copy time sampling
        out_points.set_time_sampling(points.getTimeSamplingIndex());
        
        for i in 0..points.getNumSamples() {
            if let Ok(sample) = points.getSample(i) {
                let mut out_sample = OPointsSample::new(
                    sample.positions.clone(),
                    sample.ids.clone(),
                );
                // Wrap in Some if non-empty
                if !sample.velocities.is_empty() {
                    out_sample.velocities = Some(sample.velocities.clone());
                }
                if !sample.widths.is_empty() {
                    out_sample.widths = Some(sample.widths.clone());
                }
                out_points.add_sample(&out_sample);
            }
        }
        
        return Some(out_points.build());
    }
    None
}

fn copy2_camera(obj: &IObject, _archive: &AbcIArchive, stats: &mut CopyStats) -> Option<OObject> {
    let name = obj.getName();
    if let Some(cam) = ICamera::new(obj) {
        stats.camera += 1;
        let mut out_cam = OCamera::new(name);
        
        // Copy time sampling
        out_cam.set_time_sampling(cam.getTimeSamplingIndex());
        
        for i in 0..cam.getNumSamples() {
            if let Ok(sample) = cam.getSample(i) {
                out_cam.add_sample(sample);
            }
        }
        
        return Some(out_cam.build());
    }
    None
}

fn copy2_nupatch(obj: &IObject, _archive: &AbcIArchive, stats: &mut CopyStats) -> Option<OObject> {
    let name = obj.getName();
    if let Some(nup) = INuPatch::new(obj) {
        stats.nupatch += 1;
        let mut out_nup = ONuPatch::new(name);
        
        // Copy time sampling
        out_nup.set_time_sampling(nup.getTimeSamplingIndex());
        
        for i in 0..nup.getNumSamples() {
            if let Ok(sample) = nup.getSample(i) {
                // Input uses u_knots/v_knots, output uses u_knot/v_knot
                let mut out_sample = ONuPatchSample::new(
                    sample.positions.clone(),
                    sample.num_u,
                    sample.num_v,
                    sample.u_order,
                    sample.v_order,
                    sample.u_knots.clone(),
                    sample.v_knots.clone(),
                );
                // Copy optional fields
                out_sample.position_weights = sample.position_weights.clone();
                out_sample.velocities = sample.velocities.clone();
                out_sample.uvs = sample.uvs.clone();
                out_sample.normals = sample.normals.clone();
                out_nup.add_sample(&out_sample);
            }
        }
        
        return Some(out_nup.build());
    }
    None
}

fn copy2_light(obj: &IObject, _archive: &AbcIArchive, stats: &mut CopyStats) -> Option<OObject> {
    let name = obj.getName();
    if let Some(light) = ILight::new(obj) {
        stats.light += 1;
        let mut out_light = OLight::new(name);
        
        // Copy time sampling
        out_light.set_time_sampling(light.getTimeSamplingIndex());
        
        // Light contains camera-like samples directly
        for i in 0..light.getNumSamples() {
            if let Ok(sample) = light.getSample(i) {
                out_light.add_camera_sample(sample.camera);
            }
        }
        
        return Some(out_light.build());
    }
    None
}

fn copy2_faceset(obj: &IObject, _archive: &AbcIArchive, stats: &mut CopyStats) -> Option<OObject> {
    let name = obj.getName();
    if let Some(fs) = IFaceSet::new(obj) {
        stats.faceset += 1;
        let mut out_fs = OFaceSet::new(name);
        
        // Copy time sampling
        out_fs.set_time_sampling(fs.getTimeSamplingIndex());
        
        for i in 0..fs.getNumSamples() {
            if let Ok(sample) = fs.getSample(i) {
                let out_sample = OFaceSetSample::new(sample.faces.clone());
                out_fs.add_sample(&out_sample);
            }
        }
        
        return Some(out_fs.build());
    }
    None
}
