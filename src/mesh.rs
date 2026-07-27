use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptimizedMesh {
    pub source: PathBuf,
    pub output: PathBuf,
}

pub fn optimize_mesh_to(source: &Path, output: &Path) -> std::io::Result<OptimizedMesh> {
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::copy(source, output)?;
    Ok(OptimizedMesh {
        source: source.to_path_buf(),
        output: output.to_path_buf(),
    })
}
