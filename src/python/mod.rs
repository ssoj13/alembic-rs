//! Python bindings for alembic-rs.
//!
//! Provides a Python API compatible with the original Alembic Python bindings.
//!
//! ## Usage from Python
//!
//! ```python
//! from alembic import Abc, AbcGeom
//!
//! # Open archive
//! archive = Abc.IArchive("scene.abc")
//! root = archive.getTop()
//!
//! # Navigate hierarchy
//! for child in root.children:
//!     print(child.getName())
//!
//! # Get mesh data
//! mesh_obj = root.getChildByName("mesh")
//! sample = mesh_obj.getPolyMeshSample(0)
//! print(f"Vertices: {len(sample.positions)}")
//! ```

use pyo3::prelude::*;

mod archive;
mod object;
mod time_sampling;
mod geom;
mod properties;
mod write;
mod materials;
mod schemas;

pub use archive::*;
pub use object::*;
pub use time_sampling::*;
pub use geom::*;
pub use properties::*;
pub use write::*;
pub use materials::*;
pub use schemas::*;

/// Open ABC file in 3D viewer.
///
/// # Example
/// ```python
/// import alembic_rs
/// alembic_rs.view("model.abc")
/// ```
#[cfg(feature = "viewer")]
#[pyfunction]
fn view(path: &str) -> PyResult<()> {
    let file = std::path::PathBuf::from(path);
    crate::viewer::run(Some(file))
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Viewer error: {}", e)))
}

/// Alembic Python module.
#[pymodule]
fn alembic_rs(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Register Abc submodule (core types)
    let abc = PyModule::new(m.py(), "Abc")?;
    abc.add_class::<archive::PyIArchive>()?;
    abc.add_class::<object::PyIObject>()?;
    abc.add_class::<time_sampling::PyTimeSampling>()?;
    abc.add_class::<time_sampling::PyISampleSelector>()?;
    m.add_submodule(&abc)?;
    
    // Register AbcGeom submodule (geometry schemas)
    let abc_geom = PyModule::new(m.py(), "AbcGeom")?;
    
    // ========================================================================
    // GeometryScope constants (matches original Alembic API)
    // ========================================================================
    abc_geom.add("kConstantScope", 0u8)?;
    abc_geom.add("kUniformScope", 1u8)?;
    abc_geom.add("kVaryingScope", 2u8)?;
    abc_geom.add("kVertexScope", 3u8)?;
    abc_geom.add("kFacevaryingScope", 4u8)?;
    abc_geom.add("kUnknownScope", 127u8)?;
    
    // ========================================================================
    // CurveType constants
    // ========================================================================
    // C++ ref: CurveType.h — kCubic=0, kLinear=1, kVariableOrder=2
    abc_geom.add("kCubic", 0u8)?;
    abc_geom.add("kLinear", 1u8)?;
    abc_geom.add("kVariableOrder", 2u8)?;
    
    // ========================================================================
    // CurvePeriodicity constants
    // ========================================================================
    abc_geom.add("kNonPeriodic", 0u8)?;
    abc_geom.add("kPeriodic", 1u8)?;
    
    // ========================================================================
    // BasisType constants
    // ========================================================================
    abc_geom.add("kNoBasis", 0u8)?;
    abc_geom.add("kBezierBasis", 1u8)?;
    abc_geom.add("kBsplineBasis", 2u8)?;
    abc_geom.add("kCatmullRomBasis", 3u8)?;
    abc_geom.add("kHermiteBasis", 4u8)?;
    abc_geom.add("kPowerBasis", 5u8)?;
    
    // ========================================================================
    // SubDScheme constants
    // ========================================================================
    abc_geom.add("kCatmullClark", 0u8)?;
    abc_geom.add("kLoop", 1u8)?;
    abc_geom.add("kBilinear", 2u8)?;
    
    // ========================================================================
    // TopologyVariance constants
    // ========================================================================
    abc_geom.add("kHeterogeneous", 0u8)?;
    abc_geom.add("kHomogeneous", 1u8)?;
    abc_geom.add("kStatic", 2u8)?;
    
    // ========================================================================
    // FaceSetExclusivity constants
    // ========================================================================
    abc_geom.add("kFaceSetNonExclusive", 0u8)?;
    abc_geom.add("kFaceSetExclusive", 1u8)?;
    
    // ========================================================================
    // ObjectVisibility constants
    // ========================================================================
    abc_geom.add("kVisibilityDeferred", -1i8)?;
    abc_geom.add("kVisibilityHidden", 0i8)?;
    abc_geom.add("kVisibilityVisible", 1i8)?;
    abc_geom.add_class::<geom::PyPolyMeshSample>()?;
    abc_geom.add_class::<geom::PySubDSample>()?;
    abc_geom.add_class::<geom::PyCurvesSample>()?;
    abc_geom.add_class::<geom::PyPointsSample>()?;
    abc_geom.add_class::<geom::PyCameraSample>()?;
    abc_geom.add_class::<geom::PyXformSample>()?;
    abc_geom.add_class::<geom::PyLightSample>()?;
    abc_geom.add_class::<geom::PyNuPatchSample>()?;
    abc_geom.add_class::<geom::PyFaceSetSample>()?;
    abc_geom.add_class::<geom::PyIFaceSet>()?;
    abc_geom.add_class::<geom::PyGeomParamSample>()?;
    abc_geom.add_class::<geom::PyIGeomParam>()?;
    abc_geom.add_class::<geom::PyObjectVisibility>()?;
    abc_geom.add_class::<geom::PyOVisibilityProperty>()?;
    // Schema reader classes (original Alembic API style)
    abc_geom.add_class::<schemas::PyIPolyMesh>()?;
    abc_geom.add_class::<schemas::PyIPolyMeshSchema>()?;
    abc_geom.add_class::<schemas::PyIXform>()?;
    abc_geom.add_class::<schemas::PyIXformSchema>()?;
    abc_geom.add_class::<schemas::PyISubD>()?;
    abc_geom.add_class::<schemas::PyISubDSchema>()?;
    abc_geom.add_class::<schemas::PyICurves>()?;
    abc_geom.add_class::<schemas::PyICurvesSchema>()?;
    abc_geom.add_class::<schemas::PyIPoints>()?;
    abc_geom.add_class::<schemas::PyIPointsSchema>()?;
    abc_geom.add_class::<schemas::PyICamera>()?;
    abc_geom.add_class::<schemas::PyICameraSchema>()?;
    abc_geom.add_class::<schemas::PyILight>()?;
    abc_geom.add_class::<schemas::PyILightSchema>()?;
    abc_geom.add_class::<schemas::PyINuPatch>()?;
    abc_geom.add_class::<schemas::PyINuPatchSchema>()?;
    abc_geom.add_class::<schemas::PyIFaceSetTyped>()?;
    abc_geom.add_class::<schemas::PyIFaceSetSchema>()?;
    m.add_submodule(&abc_geom)?;
    
    // Register property classes
    abc.add_class::<properties::PyICompoundProperty>()?;
    abc.add_class::<properties::PyPropertyInfo>()?;
    
    // Register write classes in Abc module
    abc.add_class::<write::PyOArchive>()?;
    abc.add_class::<write::PyOObject>()?;
    abc.add_class::<write::PyOPolyMesh>()?;
    abc.add_class::<write::PyOXform>()?;
    abc.add_class::<write::PyOCurves>()?;
    abc.add_class::<write::PyOPoints>()?;
    abc.add_class::<write::PyOSubD>()?;
    abc.add_class::<write::PyOCamera>()?;
    abc.add_class::<write::PyONuPatch>()?;
    abc.add_class::<write::PyOLight>()?;
    abc.add_class::<write::PyOFaceSet>()?;
    abc.add_class::<write::PyOMaterial>()?;
    abc.add_class::<write::PyOCollections>()?;
    abc.add_class::<write::PyOScalarProperty>()?;
    abc.add_class::<write::PyOArrayProperty>()?;
    abc.add_class::<write::PyOCompoundProperty>()?;
    
    // Register materials/collections classes
    abc.add_class::<materials::PyCollection>()?;
    abc.add_class::<materials::PyICollections>()?;
    abc.add_class::<materials::PyIMaterial>()?;
    
    // Viewer function
    #[cfg(feature = "viewer")]
    m.add_function(wrap_pyfunction!(view, m)?)?;
    
    // Also register at top level for convenience
    m.add_class::<archive::PyIArchive>()?;
    m.add_class::<object::PyIObject>()?;
    m.add_class::<time_sampling::PyTimeSampling>()?;
    m.add_class::<time_sampling::PyISampleSelector>()?;
    m.add_class::<write::PyOArchive>()?;
    m.add_class::<write::PyOPolyMesh>()?;
    m.add_class::<write::PyOXform>()?;
    // Schema readers at top level
    m.add_class::<schemas::PyIPolyMesh>()?;
    m.add_class::<schemas::PyIXform>()?;
    m.add_class::<schemas::PyISubD>()?;
    m.add_class::<schemas::PyICurves>()?;
    m.add_class::<schemas::PyIPoints>()?;
    m.add_class::<schemas::PyICamera>()?;
    m.add_class::<schemas::PyILight>()?;
    m.add_class::<schemas::PyINuPatch>()?;
    m.add_class::<schemas::PyIFaceSetTyped>()?;
    m.add_class::<schemas::PyIFaceSetSchema>()?;
    // Samples at top level
    m.add_class::<geom::PyPolyMeshSample>()?;
    m.add_class::<geom::PyXformSample>()?;
    m.add_class::<geom::PySubDSample>()?;
    m.add_class::<geom::PyCurvesSample>()?;
    m.add_class::<geom::PyPointsSample>()?;
    m.add_class::<geom::PyCameraSample>()?;
    m.add_class::<geom::PyLightSample>()?;
    m.add_class::<geom::PyNuPatchSample>()?;
    m.add_class::<geom::PyFaceSetSample>()?;
    m.add_class::<geom::PyIFaceSet>()?;
    m.add_class::<geom::PyIGeomParam>()?;
    m.add_class::<geom::PyObjectVisibility>()?;
    
    Ok(())
}
