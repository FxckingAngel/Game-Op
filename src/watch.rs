use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetRoute {
    Texture,
    Mesh,
    UnityBundle,
    Passthrough,
}

pub fn route_path(path: &Path) -> AssetRoute {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    match ext.as_str() {
        "png" | "jpg" | "jpeg" | "webp" => AssetRoute::Texture,
        "obj" => AssetRoute::Mesh,
        "bundle" | "vrca" | "vrcw" | "assets" | "resource" => AssetRoute::UnityBundle,
        _ => AssetRoute::Passthrough,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn routes_asset_paths_for_live_watchers() {
        assert_eq!(
            route_path(Path::new("avatar_face.png")),
            AssetRoute::Texture
        );
        assert_eq!(route_path(Path::new("avatar.obj")), AssetRoute::Mesh);
        assert_eq!(route_path(Path::new("world.vrcw")), AssetRoute::UnityBundle);
        assert_eq!(route_path(Path::new("voice.ogg")), AssetRoute::Passthrough);
    }
}
