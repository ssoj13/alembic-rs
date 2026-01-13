//! Alembic Viewer - 3D viewer for .abc files

use std::path::PathBuf;
use anyhow::Result;

fn main() -> Result<()> {
    let initial_file: Option<PathBuf> = std::env::args().nth(1).map(PathBuf::from);
    alembic_viewer::run(initial_file)
}
