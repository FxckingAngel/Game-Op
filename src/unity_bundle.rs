use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const SIGNATURES: &[&[u8]] = &[b"UnityFS", b"UnityWeb", b"UnityRaw"];
const SCAN_LIMIT: usize = 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnityBundleFormat {
    UnityFs,
    UnityWeb,
    UnityRaw,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BundleAssetKind {
    Texture,
    Mesh,
    Material,
    Shader,
    Audio,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundleAsset {
    pub kind: BundleAssetKind,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundleInspection {
    pub path: PathBuf,
    pub format: UnityBundleFormat,
    pub assets: Vec<BundleAsset>,
}

pub fn detect_bundle_format(path: &Path) -> io::Result<Option<UnityBundleFormat>> {
    let bytes = fs::read(path)?;
    Ok(detect_bundle_format_bytes(path, &bytes))
}

pub fn detect_bundle_format_bytes(path: &Path, bytes: &[u8]) -> Option<UnityBundleFormat> {
    let by_signature = if bytes.starts_with(SIGNATURES[0]) {
        Some(UnityBundleFormat::UnityFs)
    } else if bytes.starts_with(SIGNATURES[1]) {
        Some(UnityBundleFormat::UnityWeb)
    } else if bytes.starts_with(SIGNATURES[2]) {
        Some(UnityBundleFormat::UnityRaw)
    } else {
        None
    };

    by_signature
        .or_else(|| looks_like_vrchat_cache_bundle(path).then_some(UnityBundleFormat::UnityFs))
}

pub fn looks_like_vrchat_cache_bundle(path: &Path) -> bool {
    let lower = path
        .to_string_lossy()
        .replace('\\', "/")
        .to_ascii_lowercase();
    let ext = path
        .extension()
        .and_then(OsStr::to_str)
        .unwrap_or_default()
        .to_ascii_lowercase();

    matches!(ext.as_str(), "vrca" | "bundle" | "unity3d" | "ab")
        || lower.contains("vrchat/vrchat/cache-windowsplayer")
        || lower.contains("vrchat/cache-windowsplayer")
        || lower.contains("appdata/locallow/vrchat/vrchat/cache")
}

pub fn inspect_bundle(path: &Path) -> io::Result<Option<BundleInspection>> {
    let bytes = fs::read(path)?;
    let Some(format) = detect_bundle_format_bytes(path, &bytes) else {
        return Ok(None);
    };
    let assets = inspect_bundle_bytes(&bytes);
    Ok(Some(BundleInspection {
        path: path.to_path_buf(),
        format,
        assets,
    }))
}

pub fn inspect_bundle_bytes(bytes: &[u8]) -> Vec<BundleAsset> {
    let text = String::from_utf8_lossy(&bytes[..bytes.len().min(SCAN_LIMIT)]);
    let mut assets = Vec::new();
    for token in text.split(|c: char| {
        c.is_whitespace() || matches!(c, '\0' | '"' | '\'' | ',' | ';' | '(' | ')' | '[' | ']')
    }) {
        if let Some(kind) = classify_asset_name(token) {
            assets.push(BundleAsset {
                kind,
                name: token.trim_matches('/').to_string(),
            });
        }
    }
    assets.sort_by(|a, b| (kind_rank(a.kind), &a.name).cmp(&(kind_rank(b.kind), &b.name)));
    assets.dedup();
    assets
}

pub fn classify_asset_name(name: &str) -> Option<BundleAssetKind> {
    let lower = name.to_ascii_lowercase();
    let ext = Path::new(&lower).extension().and_then(OsStr::to_str)?;
    match ext {
        "png" | "jpg" | "jpeg" | "tga" | "dds" | "ktx" | "exr" => Some(BundleAssetKind::Texture),
        "fbx" | "obj" | "mesh" => Some(BundleAssetKind::Mesh),
        "mat" | "material" => Some(BundleAssetKind::Material),
        "shader" | "cginc" | "hlsl" => Some(BundleAssetKind::Shader),
        "wav" | "ogg" | "mp3" | "aiff" | "aif" => Some(BundleAssetKind::Audio),
        _ => None,
    }
}

fn kind_rank(kind: BundleAssetKind) -> u8 {
    match kind {
        BundleAssetKind::Texture => 0,
        BundleAssetKind::Mesh => 1,
        BundleAssetKind::Material => 2,
        BundleAssetKind::Shader => 3,
        BundleAssetKind::Audio => 4,
    }
}

pub fn write_inspection_report(
    inspection: &BundleInspection,
    asset_output: &Path,
) -> io::Result<PathBuf> {
    let stem = inspection
        .path
        .file_name()
        .and_then(OsStr::to_str)
        .unwrap_or("bundle");
    let out = asset_output
        .join("unity_bundles")
        .join(format!("{stem}.inspection.txt"));
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut body = format!(
        "source: {}\nformat: {:?}\nassets:\n",
        inspection.path.display(),
        inspection.format
    );
    for asset in &inspection.assets {
        body.push_str(&format!("- {:?}: {}\n", asset.kind, asset.name));
    }
    fs::write(&out, body)?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn detects_unityfs_signature() {
        assert_eq!(
            detect_bundle_format_bytes(Path::new("avatar.vrca"), b"UnityFS\0x"),
            Some(UnityBundleFormat::UnityFs)
        );
    }
    #[test]
    fn detects_vrchat_cache_paths() {
        assert!(looks_like_vrchat_cache_bundle(Path::new(
            "C:/Users/me/AppData/LocalLow/VRChat/VRChat/Cache-WindowsPlayer/file"
        )));
    }
    #[test]
    fn mock_bundle_lists_supported_assets() {
        let assets = inspect_bundle_bytes(b"UnityFS\0 tex/body.png mesh/avatar.fbx material/skin.mat shader/poiyomi.shader audio/voice.ogg readme.txt");
        assert_eq!(assets.len(), 5);
        assert!(assets
            .iter()
            .any(|a| a.kind == BundleAssetKind::Texture && a.name == "tex/body.png"));
    }
}
