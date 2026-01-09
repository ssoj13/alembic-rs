//! Python bindings for alembic-rs.
//!
//! Provides a Python API compatible with the original Alembic Python bindings.

use pyo3::prelude::*;

mod archive;
mod object;

pub use archive::*;
pub use object::*;

/// Alembic Python module.
#[pymodule]
fn alembic(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Register Abc submodule (core types)
    let abc = PyModule::new(m.py(), "Abc")?;
    abc.add_class::<archive::PyIArchive>()?;
    abc.add_class::<object::PyIObject>()?;
    m.add_submodule(&abc)?;
    
    Ok(())
}
