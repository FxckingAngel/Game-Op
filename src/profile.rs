#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileDefaults {
    pub target: Option<String>,
    pub quality_preset: String,
    pub max_texture_size: u32,
    pub live_asset_pass_seconds: u64,
}

impl ProfileDefaults {
    pub fn from_name(name: Option<&str>) -> Self {
        match name.unwrap_or("vrchat-hq-low-end") {
            "vrchat-hq-low-end" => Self {
                target: Some(default_vrchat_target().to_string()),
                quality_preset: "high-quality-low-end".to_string(),
                max_texture_size: 1024,
                live_asset_pass_seconds: 15,
            },
            "vrchat-performance" => Self {
                target: Some(default_vrchat_target().to_string()),
                quality_preset: "performance".to_string(),
                max_texture_size: 768,
                live_asset_pass_seconds: 10,
            },
            _ => Self {
                target: None,
                quality_preset: "high-quality-low-end".to_string(),
                max_texture_size: 1024,
                live_asset_pass_seconds: 30,
            },
        }
    }
}

fn default_vrchat_target() -> &'static str {
    if cfg!(target_os = "windows") {
        "VRChat.exe"
    } else {
        "VRChat"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn vrchat_hq_low_end_profile_sets_quality_defaults() {
        let profile = ProfileDefaults::from_name(Some("vrchat-hq-low-end"));
        assert_eq!(profile.quality_preset, "high-quality-low-end");
        assert_eq!(profile.max_texture_size, 1024);
        assert_eq!(profile.live_asset_pass_seconds, 15);
    }
}
