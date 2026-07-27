use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptimizedTexture {
    pub source: PathBuf,
    pub output: PathBuf,
}

pub fn optimize_texture_to(source: &Path, output: &Path) -> std::io::Result<OptimizedTexture> {
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::copy(source, output)?;
    Ok(OptimizedTexture {
        source: source.to_path_buf(),
        output: output.to_path_buf(),
    })
}
