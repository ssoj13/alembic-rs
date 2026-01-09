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

pub use archive::*;
pub use object::*;
pub use time_sampling::*;
pub use geom::*;

/// Alembic Python module.
#[pymodule]
fn alembic(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Register Abc submodule (core types)
    let abc = PyModule::new(m.py(), "Abc")?;
    abc.add_class::<archive::PyIArchive>()?;
    abc.add_class::<object::PyIObject>()?;
    abc.add_class::<time_sampling::PyTimeSampling>()?;
    m.add_submodule(&abc)?;
    
    // Register AbcGeom submodule (geometry schemas)
    let abc_geom = PyModule::new(m.py(), "AbcGeom")?;
    abc_geom.add_class::<geom::PyPolyMeshSample>()?;
    abc_geom.add_class::<geom::PySubDSample>()?;
    abc_geom.add_class::<geom::PyCurvesSample>()?;
    abc_geom.add_class::<geom::PyPointsSample>()?;
    abc_geom.add_class::<geom::PyCameraSample>()?;
    abc_geom.add_class::<geom::PyXformSample>()?;
    m.add_submodule(&abc_geom)?;
    
    // Also register at top level for convenience
    m.add_class::<archive::PyIArchive>()?;
    m.add_class::<object::PyIObject>()?;
    m.add_class::<time_sampling::PyTimeSampling>()?;
    
    Ok(())
}
