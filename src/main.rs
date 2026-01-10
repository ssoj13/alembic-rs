//! Alembic CLI - Tool for inspecting and manipulating Alembic files.

use alembic::prelude::{IObject, IPolyMesh, ISubD, ICurves, IPoints, ICamera, IXform};
use alembic::abc::IArchive as AbcIArchive;
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
    
    let mut xform_count = 0;
    let mut mesh_count = 0;
    let mut subd_count = 0;
    let mut curve_count = 0;
    let mut point_count = 0;
    let mut camera_count = 0;
    let mut light_count = 0;
    let mut other_count = 0;
    let mut total_vertices = 0usize;
    let mut total_faces = 0usize;
    
    count_objects(&root, &mut xform_count, &mut mesh_count, &mut subd_count,
                  &mut curve_count, &mut point_count, &mut camera_count,
                  &mut light_count, &mut other_count, 
                  &mut total_vertices, &mut total_faces);
    
    debug!("Counted {} total objects", xform_count + mesh_count + subd_count + curve_count + point_count + camera_count + light_count + other_count);
    
    println!("Objects:");
    println!("  Xforms:  {}", xform_count);
    println!("  Meshes:  {} ({} vertices, {} faces)", mesh_count, total_vertices, total_faces);
    println!("  SubDs:   {}", subd_count);
    println!("  Curves:  {}", curve_count);
    println!("  Points:  {}", point_count);
    println!("  Cameras: {}", camera_count);
    println!("  Lights:  {}", light_count);
    if other_count > 0 {
        println!("  Other:   {}", other_count);
    }
    println!();
    println!("Total objects: {}", xform_count + mesh_count + subd_count + curve_count + point_count + camera_count + light_count + other_count);
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

fn count_objects(
    obj: &IObject,
    xform: &mut usize,
    mesh: &mut usize,
    subd: &mut usize,
    curve: &mut usize,
    point: &mut usize,
    camera: &mut usize,
    light: &mut usize,
    other: &mut usize,
    total_verts: &mut usize,
    total_faces: &mut usize,
) {
    let schema = obj.meta_data().get("schema").unwrap_or_default();
    let schema_str: &str = &schema;
    trace!("Processing object: {} [{}]", obj.name(), schema);
    
    if schema_str.contains("Xform") {
        *xform += 1;
    } else if schema_str.contains("PolyMesh") {
        *mesh += 1;
        if let Some(poly) = IPolyMesh::new(obj) {
            if let Ok(sample) = poly.get_sample(0) {
                *total_verts += sample.positions.len();
                *total_faces += sample.face_counts.len();
            }
        }
    } else if schema_str.contains("SubD") {
        *subd += 1;
        if let Some(sd) = ISubD::new(obj) {
            if let Ok(sample) = sd.get_sample(0) {
                *total_verts += sample.positions.len();
                *total_faces += sample.face_counts.len();
            }
        }
    } else if schema_str.contains("Curves") {
        *curve += 1;
    } else if schema_str.contains("Points") {
        *point += 1;
    } else if schema_str.contains("Camera") {
        *camera += 1;
    } else if schema_str.contains("Light") {
        *light += 1;
    } else if !schema_str.is_empty() {
        *other += 1;
    }
    
    for i in 0..obj.num_children() {
        if let Some(child) = obj.child(i) {
            count_objects(&child, xform, mesh, subd, curve, point, camera, light, other, total_verts, total_faces);
        }
    }
}

fn print_tree(obj: &IObject, depth: usize) {
    let indent = "  ".repeat(depth);
    let schema = obj.meta_data().get("schema").unwrap_or_default();
    let type_str = schema_to_type(&schema);
    
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
    let type_str = schema_to_type(&schema);
    
    // Get additional info based on type
    let extra_info = get_object_info(obj, &schema);
    
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
