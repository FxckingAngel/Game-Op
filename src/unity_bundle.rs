use std::{fs, io, path::Path};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnityBundleInfo {
    pub path: String,
    pub signature: String,
    pub supported_for_rewrite: bool,
}

pub fn inspect_bundle(path: &Path) -> io::Result<Option<UnityBundleInfo>> {
    let mut bytes = [0u8; 128];
    let mut file = fs::File::open(path)?;
    use std::io::Read;
    let read = file.read(&mut bytes)?;
    if read < 12 {
        return Ok(None);
    }
    
    let sig_bytes = &bytes[0..7];
    let signature = if sig_bytes.starts_with(b"UnityFS") {
        "UnityFS"
    } else if sig_bytes.starts_with(b"UnityWeb") {
        "UnityWeb"
    } else if sig_bytes.starts_with(b"UnityRaw") {
        "UnityRaw"
    } else {
        return Ok(None);
    };

    let mut version = String::new();
    if read > 12 {
        let version_bytes = &bytes[12..];
        if let Some(null_pos) = version_bytes.iter().position(|&b| b == 0) {
            let ver_str = String::from_utf8_lossy(&version_bytes[..null_pos]);
            if !ver_str.is_empty() && ver_str.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == 'f') {
                version = ver_str.to_string();
            }
        }
    }

    let signature_with_version = if version.is_empty() {
        signature.to_string()
    } else {
        format!("{} - Unity {}", signature, version)
    };

    Ok(Some(UnityBundleInfo {
        path: path.display().to_string(),
        signature: signature_with_version,
        supported_for_rewrite: false,
    }))
}

pub fn is_probable_bundle(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    
    if name == "__data" {
        return true;
    }

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

    #[test]
    fn extracts_unity_version() {
        let path = std::env::temp_dir().join(format!(
            "bundle-ver-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        // Signature (8 bytes), Format (4 bytes), Version (null-terminated string)
        let data = b"UnityFS\0\0\0\0\x0d2022.3.22f1\0SomeBinary".to_vec();
        fs::write(&path, data).unwrap();
        let info = inspect_bundle(&path).unwrap().unwrap();
        assert_eq!(info.signature, "UnityFS - Unity 2022.3.22f1");
        fs::remove_file(path).unwrap();
    }
}
