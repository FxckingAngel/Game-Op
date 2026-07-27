use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::mesh;
use crate::texture;
use crate::unity_bundle::{self, BundleInspection};

#[derive(Debug, Clone)]
pub struct AssetStager {
    asset_output: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StagedAsset {
    Texture(PathBuf),
    Mesh(PathBuf),
    UnityBundle {
        inspection: BundleInspection,
        report: PathBuf,
    },
    Passthrough(PathBuf),
}

impl AssetStager {
    pub fn new(asset_output: impl Into<PathBuf>) -> Self {
        Self {
            asset_output: asset_output.into(),
        }
    }

    pub fn stage_asset(&self, source: &Path) -> io::Result<StagedAsset> {
        if let Some(inspection) = unity_bundle::inspect_bundle(source)? {
            let report = unity_bundle::write_inspection_report(&inspection, &self.asset_output)?;
            return Ok(StagedAsset::UnityBundle { inspection, report });
        }

        let output = self
            .asset_output
            .join(source.file_name().unwrap_or_default());
        match source
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str()
        {
            "png" | "jpg" | "jpeg" | "tga" | "dds" => Ok(StagedAsset::Texture(
                texture::optimize_texture_to(source, &output)?.output,
            )),
            "fbx" | "obj" | "mesh" => Ok(StagedAsset::Mesh(
                mesh::optimize_mesh_to(source, &output)?.output,
            )),
            _ => {
                if let Some(parent) = output.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::copy(source, &output)?;
                Ok(StagedAsset::Passthrough(output))
            }
        }
    }

    pub fn asset_output(&self) -> &Path {
        &self.asset_output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn stages_bundle_by_writing_report_without_overwriting_original() {
        let root = std::env::temp_dir().join(format!(
            "game-op-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let src = root.join("avatar.vrca");
        let out = root.join("out");
        fs::create_dir_all(&root).unwrap();
        fs::write(&src, b"UnityFS\0 Assets/body.png Assets/avatar.fbx").unwrap();

        let staged = AssetStager::new(&out).stage_asset(&src).unwrap();
        match staged {
            StagedAsset::UnityBundle { inspection, report } => {
                assert_eq!(inspection.assets.len(), 2);
                assert!(report.starts_with(&out));
                assert_eq!(
                    fs::read(&src).unwrap(),
                    b"UnityFS\0 Assets/body.png Assets/avatar.fbx"
                );
            }
            other => panic!("unexpected staged asset: {other:?}"),
        }
        let _ = fs::remove_dir_all(root);
    }
}
