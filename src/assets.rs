use std::{
    fs, io,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone)]
pub struct AssetStagePolicy {
    pub cache_dir: Option<String>,
    pub output_dir: Option<String>,
    pub dry_run: bool,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct AssetStageReport {
    pub scanned: usize,
    pub staged: usize,
    pub skipped_optimized_types: usize,
}

pub struct AssetStager {
    policy: AssetStagePolicy,
}

impl AssetStager {
    pub fn new(policy: AssetStagePolicy) -> Self {
        Self { policy }
    }

    pub fn stage_unoptimized_assets(&self) -> io::Result<AssetStageReport> {
        let Some(cache_dir) = &self.policy.cache_dir else {
            return Ok(AssetStageReport::default());
        };
        let input_root = PathBuf::from(cache_dir);
        if !input_root.exists() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("asset cache does not exist: {}", input_root.display()),
            ));
        }
        let output_root = self
            .policy
            .output_dir
            .as_ref()
            .map(PathBuf::from)
            .unwrap_or_else(|| input_root.with_extension("game-op-assets-optimized"));
        let mut report = AssetStageReport::default();
        self.walk(&input_root, &input_root, &output_root, &mut report)?;
        println!(
            "Asset stage complete: scanned={}, staged={}, skipped_optimized_types={}, output={}",
            report.scanned,
            report.staged,
            report.skipped_optimized_types,
            output_root.display()
        );
        Ok(report)
    }

    fn walk(
        &self,
        input_root: &Path,
        current: &Path,
        output_root: &Path,
        report: &mut AssetStageReport,
    ) -> io::Result<()> {
        for entry in fs::read_dir(current)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                self.walk(input_root, &path, output_root, report)?;
                continue;
            }
            report.scanned += 1;
            if is_optimized_by_specialized_pass(&path) {
                report.skipped_optimized_types += 1;
                continue;
            }
            let relative = path.strip_prefix(input_root).unwrap_or(&path);
            let output = output_root.join(relative);
            if self.policy.dry_run {
                println!("dry-run: would stage passthrough asset {}", path.display());
            } else {
                if let Some(parent) = output.parent() {
                    fs::create_dir_all(parent)?;
                }
                copy_if_newer(&path, &output)?;
            }
            report.staged += 1;
        }
        Ok(())
    }
}

fn copy_if_newer(input: &Path, output: &Path) -> io::Result<()> {
    let should_copy = match (fs::metadata(input), fs::metadata(output)) {
        (Ok(input_meta), Ok(output_meta)) => {
            input_meta.modified().ok() > output_meta.modified().ok()
                || input_meta.len() != output_meta.len()
        }
        (Ok(_), Err(_)) => true,
        _ => false,
    };
    if should_copy {
        fs::copy(input, output)?;
    }
    Ok(())
}

fn is_optimized_by_specialized_pass(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "png" | "jpg" | "jpeg" | "webp" | "obj"
            )
        })
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn stages_unknown_assets_and_leaves_textures_for_texture_pass() {
        let root = temp_path("game-op-asset-stage-test");
        let input = root.join("cache");
        let output = root.join("optimized");
        fs::create_dir_all(&input).unwrap();
        fs::write(input.join("avatar.bundle"), b"bundle").unwrap();
        fs::write(input.join("avatar.png"), b"not-staged-here").unwrap();

        let stager = AssetStager::new(AssetStagePolicy {
            cache_dir: Some(input.to_string_lossy().into_owned()),
            output_dir: Some(output.to_string_lossy().into_owned()),
            dry_run: false,
        });
        let report = stager.stage_unoptimized_assets().unwrap();
        assert_eq!(report.staged, 1);
        assert_eq!(report.skipped_optimized_types, 1);
        assert!(output.join("avatar.bundle").exists());
        assert!(!output.join("avatar.png").exists());
        fs::remove_dir_all(root).unwrap();
    }

    fn temp_path(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("{name}-{nanos}"))
    }
}
