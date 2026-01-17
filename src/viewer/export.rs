//! Alembic export functionality for viewer
//! 
//! Exports scene data using the Rust Alembic writer (OArchive, OPolyMesh, etc.)

use std::path::Path;
use log::{info, debug};

use crate::abc::{IArchive, IObject};
use crate::geom::{IPolyMesh, IXform, ISubD, ICurves, IPoints, ICamera, INuPatch, ILight, IFaceSet};
use crate::ogawa::{OArchive, OObject, OPolyMesh, OPolyMeshSample, OXform, OXformSample};
use crate::ogawa::{OSubD, OSubDSample, OCurves, OCurvesSample, OPoints, OPointsSample};
use crate::ogawa::{OCamera, ONuPatch, ONuPatchSample, OLight, OFaceSet, OFaceSetSample};

/// Export statistics
#[derive(Default, Debug)]
pub struct ExportStats {
    pub xform: usize,
    pub polymesh: usize,
    pub subd: usize,
    pub curves: usize,
    pub points: usize,
    pub camera: usize,
    pub nupatch: usize,
    pub light: usize,
    pub faceset: usize,
    pub other: usize,
}

impl ExportStats {
    pub fn total(&self) -> usize {
        self.xform + self.polymesh + self.subd + self.curves + 
        self.points + self.camera + self.nupatch + self.light + 
        self.faceset + self.other
    }
}

/// Export an archive to a new file using the Rust writer
pub fn export_archive(input: &IArchive, output_path: &Path) -> Result<ExportStats, String> {
    info!("Exporting to {}", output_path.display());
    
    let mut out_archive = OArchive::create(output_path)
        .map_err(|e| format!("Failed to create output: {}", e))?;
    
    // Copy archive metadata
    out_archive.set_archive_metadata(input.getArchiveMetaData().clone());
    
    // Copy time samplings (skip index 0 - identity)
    for i in 1..input.getNumTimeSamplings() {
        if let Some(ts) = input.getTimeSampling(i) {
            out_archive.addTimeSampling(ts.clone());
        }
    }
    
    let root = input.getTop();
    let mut out_root = OObject::new("");
    let mut stats = ExportStats::default();
    
    // Export children recursively
    for child in root.getChildren() {
        if let Some(out_child) = export_object(&child, input, &mut stats) {
            out_root.add_child(out_child);
        }
    }
    
    out_archive.write_archive(&out_root)
        .map_err(|e| format!("Failed to write archive: {}", e))?;
    
    info!("Export complete: {} objects", stats.total());
    Ok(stats)
}

fn export_object(obj: &IObject, archive: &IArchive, stats: &mut ExportStats) -> Option<OObject> {
    let name = obj.getName();
    let schema = obj.getMetaData().get("schema").unwrap_or_default();
    
    debug!("export: {} [{}]", name, schema);
    
    // Handle different schema types
    if schema.contains("Xform") {
        return export_xform(obj, archive, stats);
    } else if schema.contains("PolyMesh") {
        return export_polymesh(obj, archive, stats);
    } else if schema.contains("SubD") {
        return export_subd(obj, archive, stats);
    } else if schema.contains("Curve") {
        return export_curves(obj, stats);
    } else if schema.contains("Points") {
        return export_points(obj, stats);
    } else if schema.contains("Camera") {
        return export_camera(obj, stats);
    } else if schema.contains("NuPatch") {
        return export_nupatch(obj, stats);
    } else if schema.contains("Light") {
        return export_light(obj, stats);
    } else if schema.contains("FaceSet") {
        return export_faceset(obj, stats);
    }
    
    // Generic object - just export children
    stats.other += 1;
    let mut out_obj = OObject::new(name);
    for child in obj.getChildren() {
        if let Some(out_child) = export_object(&child, archive, stats) {
            out_obj.add_child(out_child);
        }
    }
    Some(out_obj)
}

fn export_xform(obj: &IObject, archive: &IArchive, stats: &mut ExportStats) -> Option<OObject> {
    let name = obj.getName();
    if let Some(xform) = IXform::new(obj) {
        stats.xform += 1;
        let mut out_xform = OXform::new(name);
        out_xform.set_time_sampling(xform.getTimeSamplingIndex());
        
        for i in 0..xform.getNumSamples() {
            if let Ok(sample) = xform.getSample(i) {
                let matrix = sample.matrix();
                out_xform.add_sample(OXformSample::from_matrix(matrix, sample.inherits));
            }
        }
        
        let mut out_obj = out_xform.build();
        for child in obj.getChildren() {
            if let Some(out_child) = export_object(&child, archive, stats) {
                out_obj.add_child(out_child);
            }
        }
        return Some(out_obj);
    }
    
    // Fallback to generic
    debug!("export_xform: IXform::new failed for {}", name);
    let mut out_obj = OObject::new(name);
    for child in obj.getChildren() {
        if let Some(out_child) = export_object(&child, archive, stats) {
            out_obj.add_child(out_child);
        }
    }
    Some(out_obj)
}

fn export_polymesh(obj: &IObject, archive: &IArchive, stats: &mut ExportStats) -> Option<OObject> {
    let name = obj.getName();
    if let Some(mesh) = IPolyMesh::new(obj) {
        stats.polymesh += 1;
        let mut out_mesh = OPolyMesh::new(name);
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
        for child in obj.getChildren() {
            if let Some(out_child) = export_object(&child, archive, stats) {
                out_obj.add_child(out_child);
            }
        }
        return Some(out_obj);
    }
    
    debug!("export_polymesh: IPolyMesh::new failed for {}", name);
    let mut out_obj = OObject::new(name);
    for child in obj.getChildren() {
        if let Some(out_child) = export_object(&child, archive, stats) {
            out_obj.add_child(out_child);
        }
    }
    Some(out_obj)
}

fn export_subd(obj: &IObject, archive: &IArchive, stats: &mut ExportStats) -> Option<OObject> {
    let name = obj.getName();
    if let Some(sd) = ISubD::new(obj) {
        stats.subd += 1;
        let mut out_sd = OSubD::new(name);
        out_sd.set_time_sampling(sd.getTimeSamplingIndex());
        
        for i in 0..sd.getNumSamples() {
            if let Ok(sample) = sd.getSample(i) {
                let mut out_sample = OSubDSample::new(
                    sample.positions.clone(),
                    sample.face_counts.clone(),
                    sample.face_indices.clone(),
                );
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
            if let Some(out_child) = export_object(&child, archive, stats) {
                out_obj.add_child(out_child);
            }
        }
        return Some(out_obj);
    }
    None
}

fn export_curves(obj: &IObject, stats: &mut ExportStats) -> Option<OObject> {
    let name = obj.getName();
    if let Some(curves) = ICurves::new(obj) {
        stats.curves += 1;
        let mut out_curves = OCurves::new(name);
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

fn export_points(obj: &IObject, stats: &mut ExportStats) -> Option<OObject> {
    let name = obj.getName();
    if let Some(points) = IPoints::new(obj) {
        stats.points += 1;
        let mut out_points = OPoints::new(name);
        out_points.set_time_sampling(points.getTimeSamplingIndex());
        
        for i in 0..points.getNumSamples() {
            if let Ok(sample) = points.getSample(i) {
                let mut out_sample = OPointsSample::new(
                    sample.positions.clone(),
                    sample.ids.clone(),
                );
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

fn export_camera(obj: &IObject, stats: &mut ExportStats) -> Option<OObject> {
    let name = obj.getName();
    if let Some(cam) = ICamera::new(obj) {
        stats.camera += 1;
        let mut out_cam = OCamera::new(name);
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

fn export_nupatch(obj: &IObject, stats: &mut ExportStats) -> Option<OObject> {
    let name = obj.getName();
    if let Some(nup) = INuPatch::new(obj) {
        stats.nupatch += 1;
        let mut out_nup = ONuPatch::new(name);
        out_nup.set_time_sampling(nup.getTimeSamplingIndex());
        
        for i in 0..nup.getNumSamples() {
            if let Ok(sample) = nup.getSample(i) {
                let mut out_sample = ONuPatchSample::new(
                    sample.positions.clone(),
                    sample.num_u,
                    sample.num_v,
                    sample.u_order,
                    sample.v_order,
                    sample.u_knots.clone(),
                    sample.v_knots.clone(),
                );
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

fn export_light(obj: &IObject, stats: &mut ExportStats) -> Option<OObject> {
    let name = obj.getName();
    if let Some(light) = ILight::new(obj) {
        stats.light += 1;
        let mut out_light = OLight::new(name);
        out_light.set_time_sampling(light.getTimeSamplingIndex());
        
        for i in 0..light.getNumSamples() {
            if let Ok(sample) = light.getSample(i) {
                out_light.add_camera_sample(sample.camera);
            }
        }
        
        return Some(out_light.build());
    }
    None
}

fn export_faceset(obj: &IObject, stats: &mut ExportStats) -> Option<OObject> {
    let name = obj.getName();
    if let Some(fs) = IFaceSet::new(obj) {
        stats.faceset += 1;
        let mut out_fs = OFaceSet::new(name);
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
