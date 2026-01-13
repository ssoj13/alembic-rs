//! Alembic CLI - Tool for inspecting and manipulating Alembic files.

use alembic::prelude::{IObject, IPolyMesh, ISubD, ICurves, IPoints, ICamera, IXform};
use alembic::abc::IArchive as AbcIArchive;
use alembic::ogawa::writer::{OArchive, OObject, OPolyMesh, OPolyMeshSample, OXform, OXformSample};
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
        print_usage(&args[0]);
        return;
    }
    
    match filtered_args[0] {
        "info" | "i" => {
            if filtered_args.len() < 2 {
                eprintln!("Usage: {} info <file.abc>", args[0]);
                std::process::exit(1);
            }
            cmd_info(filtered_args[1]);
        }
        "tree" | "t" => {
            if filtered_args.len() < 2 {
                eprintln!("Usage: {} tree <file.abc>", args[0]);
                std::process::exit(1);
            }
            cmd_tree(filtered_args[1]);
        }
        "stats" | "s" => {
            if filtered_args.len() < 2 {
                eprintln!("Usage: {} stats <file.abc>", args[0]);
                std::process::exit(1);
            }
            cmd_stats(filtered_args[1]);
        }
        "dump" | "d" => {
            if filtered_args.len() < 2 {
                eprintln!("Usage: {} dump <file.abc> [pattern] [--json]", args[0]);
                std::process::exit(1);
            }
            let json_mode = filtered_args.iter().any(|&s| s == "--json" || s == "-j");
            if json_mode {
                set_log_level(LOG_QUIET); // Suppress all logs for clean JSON
            }
            let pattern = filtered_args.get(2).filter(|&&s| s != "--json" && s != "-j").map(|s| *s);
            cmd_dump(filtered_args[1], pattern, json_mode);
        }
        "copy" | "c" => {
            if filtered_args.len() < 3 {
                eprintln!("Usage: {} copy <input.abc> <output.abc>", args[0]);
                std::process::exit(1);
            }
            cmd_copy(filtered_args[1], filtered_args[2]);
        }
        "help" | "h" | "-h" | "--help" => print_usage(&args[0]),
        _ => {
            // Assume it's a file path
            if Path::new(filtered_args[0]).exists() {
                cmd_info(filtered_args[0]);
            } else {
                eprintln!("Unknown command: {}", filtered_args[0]);
                print_usage(&args[0]);
                std::process::exit(1);
            }
        }
    }
}

fn print_usage(prog: &str) {
    println!("Alembic CLI - Inspect Alembic files");
    println!();
    println!("Usage: {} [options] <command> <file.abc>", prog);
    println!();
    println!("Commands:");
    println!("  i, info    Show archive info and object summary");
    println!("  t, tree    Show full object hierarchy");
    println!("  s, stats   Show detailed statistics");
    println!("  d, dump    Dump xform details (pattern optional)");
    println!("  c, copy    Copy archive (round-trip test)");
    println!("  h, help    Show this help");
    println!();
    println!("Options:");
    println!("  -v, --verbose  Debug output");
    println!("  -vv, --trace   Trace output (very verbose)");
    println!("  -q, --quiet    Suppress output");
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
    println!("Version: {}", archive.archive_version());
    println!("Time samplings: {}", archive.num_time_samplings());
    println!();
    
    // Count objects by type
    let root = archive.root();
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
    
    let root = archive.root();
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
    println!("Version: {}", archive.archive_version());
    println!();
    
    // Time samplings
    println!("Time Samplings ({}):", archive.num_time_samplings());
    for i in 0..archive.num_time_samplings() {
        if let Some(ts) = archive.time_sampling(i) {
            let type_str = if ts.is_identity() {
                "Identity".to_string()
            } else if ts.is_uniform() {
                format!("Uniform ({}fps)", (1.0_f64 / ts.time_per_cycle()).round())
            } else if ts.is_cyclic() {
                format!("Cyclic ({} per cycle)", ts.samples_per_cycle())
            } else {
                format!("Acyclic ({} times)", ts.num_stored_times())
            };
            
            let max_samples = archive.max_num_samples_for_time_sampling(i).unwrap_or(0);
            println!("  [{}] {} - {} samples", i, type_str, max_samples);
        }
    }
    println!();
    
    // Object stats
    let root = archive.root();
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
    let schema = obj.meta_data().get("schema").unwrap_or_default();
    let schema_str = schema;
    trace!("Processing object: {} [{}]", obj.name(), schema);
    
    if schema_str.contains("Xform") {
        counts.xform += 1;
    } else if schema_str.contains("PolyMesh") {
        counts.mesh += 1;
        if let Some(poly) = IPolyMesh::new(obj) {
            if let Ok(sample) = poly.get_sample(0) {
                counts.total_verts += sample.positions.len();
                counts.total_faces += sample.face_counts.len();
            }
        }
    } else if schema_str.contains("SubD") {
        counts.subd += 1;
        if let Some(sd) = ISubD::new(obj) {
            if let Ok(sample) = sd.get_sample(0) {
                counts.total_verts += sample.positions.len();
                counts.total_faces += sample.face_counts.len();
            }
        }
    } else if schema_str.contains("Curves") {
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
    
    for i in 0..obj.num_children() {
        if let Some(child) = obj.child(i) {
            count_objects(&child, counts);
        }
    }
}

fn print_tree(obj: &IObject, depth: usize) {
    let indent = "  ".repeat(depth);
    let schema = obj.meta_data().get("schema").unwrap_or_default();
    let type_str = schema_to_type(schema);
    
    if depth == 0 {
        println!("{}/", obj.name());
    } else {
        println!("{}{} [{}]", indent, obj.name(), type_str);
    }
    
    for i in 0..obj.num_children() {
        if let Some(child) = obj.child(i) {
            print_tree(&child, depth + 1);
        }
    }
}

fn print_stats_tree(obj: &IObject, depth: usize) {
    let indent = "  ".repeat(depth);
    let schema = obj.meta_data().get("schema").unwrap_or_default();
    let type_str = schema_to_type(schema);
    
    // Get additional info based on type
    let extra_info = get_object_info(obj, schema);
    
    if depth == 0 {
        println!("{}/", obj.name());
    } else if extra_info.is_empty() {
        println!("{}{} [{}]", indent, obj.name(), type_str);
    } else {
        println!("{}{} [{}] {}", indent, obj.name(), type_str, extra_info);
    }
    
    for i in 0..obj.num_children() {
        if let Some(child) = obj.child(i) {
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
        let root = archive.root();
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
        let root = archive.root();
        dump_xforms(&root, 0, glam::Mat4::IDENTITY, pattern);
    }
}

fn dump_xforms(obj: &IObject, depth: usize, parent_world: glam::Mat4, pattern: Option<&str>) {
    let indent = "  ".repeat(depth);
    let name = obj.name();
    let full_name = obj.full_name();
    
    // Check pattern filter
    let matches_pattern = pattern.map(|p| full_name.contains(p) || name.contains(p)).unwrap_or(true);
    
    // Get local transform
    let local_matrix = if let Some(xform) = IXform::new(obj) {
        if let Ok(sample) = xform.get_sample(0) {
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
        let schema = obj.meta_data().get("schema").unwrap_or_default();
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
    for child in obj.children() {
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
    let name = obj.name();
    let full_name = obj.full_name();
    let matches = pattern.map(|p| full_name.contains(p) || name.contains(p)).unwrap_or(true);
    
    let local_matrix = if let Some(xform) = IXform::new(obj) {
        if let Ok(sample) = xform.get_sample(0) {
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
        let schema = obj.meta_data().get("schema").unwrap_or_default();
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
    
    for child in obj.children() {
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
    
    let root = archive.root();
    let mut out_root = OObject::new("");
    
    // Copy children recursively
    for child in root.children() {
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
    let name = obj.name();
    let schema = obj.meta_data().get("schema").unwrap_or_default();
    
    debug!("Copying object: {} [{}]", name, schema);
    
    // Handle different schema types
    if schema.contains("Xform") {
        if let Some(xform) = IXform::new(obj) {
            let mut out_xform = OXform::new(name);
            
            // Copy all samples - convert ops to matrix
            for i in 0..xform.num_samples() {
                if let Ok(sample) = xform.get_sample(i) {
                    let matrix = sample.matrix();
                    out_xform.add_sample(OXformSample::from_matrix(matrix, sample.inherits));
                }
            }
            
            let mut out_obj = out_xform.build();
            
            // Copy children
            for child in obj.children() {
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
            for i in 0..mesh.num_samples() {
                if let Ok(sample) = mesh.get_sample(i) {
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
    for child in obj.children() {
        if let Some(out_child) = copy_object(&child) {
            out_obj.add_child(out_child);
        }
    }
    Some(out_obj)
}

fn get_object_info(obj: &IObject, schema: &str) -> String {
    if schema.contains("PolyMesh") {
        if let Some(poly) = IPolyMesh::new(obj) {
            let samples = poly.num_samples();
            if let Ok(sample) = poly.get_sample(0) {
                return format!("- {} verts, {} faces, {} samples", 
                    sample.positions.len(), sample.face_counts.len(), samples);
            }
        }
    } else if schema.contains("SubD") {
        if let Some(sd) = ISubD::new(obj) {
            let samples = sd.num_samples();
            if let Ok(sample) = sd.get_sample(0) {
                return format!("- {} verts, {} faces, {} samples",
                    sample.positions.len(), sample.face_counts.len(), samples);
            }
        }
    } else if schema.contains("Curves") {
        if let Some(curves) = ICurves::new(obj) {
            let samples = curves.num_samples();
            if let Ok(sample) = curves.get_sample(0) {
                return format!("- {} curves, {} samples",
                    sample.num_vertices.len(), samples);
            }
        }
    } else if schema.contains("Points") {
        if let Some(points) = IPoints::new(obj) {
            let samples = points.num_samples();
            if let Ok(sample) = points.get_sample(0) {
                return format!("- {} points, {} samples",
                    sample.positions.len(), samples);
            }
        }
    } else if schema.contains("Camera") {
        if let Some(cam) = ICamera::new(obj) {
            let samples = cam.num_samples();
            if let Ok(sample) = cam.get_sample(0) {
                return format!("- focal={:.1}mm, {} samples",
                    sample.focal_length, samples);
            }
        }
    } else if schema.contains("Xform") {
        if let Some(xf) = IXform::new(obj) {
            let samples = xf.num_samples();
            let constant = if xf.is_constant() { "static" } else { "animated" };
            return format!("- {}, {} samples", constant, samples);
        }
    }
    String::new()
}
