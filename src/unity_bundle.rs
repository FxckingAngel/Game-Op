use std::{fs, io, path::Path};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnityBundleInfo {
    pub path: String,
    pub signature: String,
    pub supported_for_rewrite: bool,
}

pub fn inspect_bundle(path: &Path) -> io::Result<Option<UnityBundleInfo>> {
    let mut bytes = [0u8; 32];
    let mut file = fs::File::open(path)?;
    use std::io::Read;
    let read = file.read(&mut bytes)?;
    let header = String::from_utf8_lossy(&bytes[..read]);
    let signature = if header.contains("UnityFS") {
        "UnityFS"
    } else if header.contains("UnityWeb") {
        "UnityWeb"
    } else if header.contains("UnityRaw") {
        "UnityRaw"
    } else {
        return Ok(None);
    };
    Ok(Some(UnityBundleInfo {
        path: path.display().to_string(),
        signature: signature.to_string(),
        supported_for_rewrite: false,
    }))
}

pub fn is_probable_bundle(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| {
            matches!(
                e.to_ascii_lowercase().as_str(),
                "bundle" | "vrca" | "vrcw" | "assets" | "resource"
            )
        })
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};
    #[test]
    fn identifies_unityfs_header() {
        let path = std::env::temp_dir().join(format!(
            "bundle-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::write(&path, b"UnityFS\0test").unwrap();
        let info = inspect_bundle(&path).unwrap().unwrap();
        assert_eq!(info.signature, "UnityFS");
        fs::remove_file(path).unwrap();
    }
}
