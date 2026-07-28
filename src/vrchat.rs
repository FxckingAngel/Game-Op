use crate::unity_bundle;
use std::{fs, io, path::Path};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct VrchatAssetReport {
    pub textures: usize,
    pub meshes: usize,
    pub probable_bundles: usize,
    pub passthrough: usize,
    pub estimated_texture_megabytes: u64,
}

pub fn analyze_cache(root: &Path, max_texture_size: u32) -> io::Result<VrchatAssetReport> {
    let mut report = VrchatAssetReport::default();
    walk(root, max_texture_size, &mut report)?;
    Ok(report)
}

fn walk(current: &Path, max_texture_size: u32, report: &mut VrchatAssetReport) -> io::Result<()> {
    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            walk(&path, max_texture_size, report)?;
            continue;
        }
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        match ext.as_str() {
            "png" | "jpg" | "jpeg" | "webp" => {
                report.textures += 1;
                report.estimated_texture_megabytes += estimate_texture_mb(max_texture_size);
            }
            "obj" => report.meshes += 1,
            _ if unity_bundle::is_probable_bundle(&path) => report.probable_bundles += 1,
            _ => report.passthrough += 1,
        }
    }
    Ok(())
}

fn estimate_texture_mb(max_side: u32) -> u64 {
    ((max_side as u64 * max_side as u64 * 4) / (1024 * 1024)).max(1)
}

pub fn print_recommendations(report: &VrchatAssetReport) {
    println!("VRChat asset report: textures={}, meshes={}, probable_bundles={}, passthrough={}, estimated_texture_budget={}MiB", report.textures, report.meshes, report.probable_bundles, report.passthrough, report.estimated_texture_megabytes);
    if report.probable_bundles > 0 {
        println!("Unity bundles detected; currently staged safely unless a supported decoder is added for rewrite.");
    }
    if report.estimated_texture_megabytes > 512 {
        println!(
            "Recommendation: try --max-texture-size 768 for Intel HD 620 shared-memory stability."
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};
    #[test]
    fn reports_vrchat_like_cache_mix() {
        let root = std::env::temp_dir().join(format!(
            "vrchat-report-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("avatar.png"), b"x").unwrap();
        fs::write(root.join("avatar.obj"), b"x").unwrap();
        fs::write(root.join("world.vrcw"), b"UnityFS").unwrap();
        let report = analyze_cache(&root, 1024).unwrap();
        assert_eq!(report.textures, 1);
        assert_eq!(report.meshes, 1);
        assert_eq!(report.probable_bundles, 1);
        fs::remove_dir_all(root).unwrap();
    }
}
