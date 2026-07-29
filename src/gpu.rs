use std::{io, process::Command};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GpuVendor {
    Intel,
    Nvidia,
    Amd,
    Apple,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuInfo {
    pub vendor: GpuVendor,
    pub name: String,
}

pub fn detect_gpus() -> io::Result<Vec<GpuInfo>> {
    if cfg!(target_os = "windows") {
        parse_gpu_names(&run(
            "powershell",
            &[
                "-NoProfile",
                "-Command",
                "Get-CimInstance Win32_VideoController | Select-Object -ExpandProperty Name",
            ],
        )?)
        .pipe(Ok)
    } else if cfg!(target_os = "macos") {
        parse_gpu_names(&run(
            "sh",
            &[
                "-c",
                "system_profiler SPDisplaysDataType | awk -F': ' '/Chipset Model/{print $2}'",
            ],
        )?)
        .pipe(Ok)
    } else {
        let output = run(
            "sh",
            &[
                "-c",
                "(command -v lspci >/dev/null && lspci | grep -Ei 'vga|3d|display') || \
                 (command -v lshw >/dev/null && lshw -C display 2>/dev/null | grep -Ei 'product|vendor') || \
                 (command -v glxinfo >/dev/null && glxinfo | grep -Ei 'OpenGL renderer string') || true",
            ],
        )?;
        parse_gpu_names(&output).pipe(Ok)
    }
}

pub fn recommendation_for(gpus: &[GpuInfo]) -> String {
    if gpus
        .iter()
        .any(|gpu| gpu.name.to_ascii_lowercase().contains("hd graphics 620"))
    {
        "Intel HD Graphics 620 detected: use --max-texture-size 1024 for quality, 768 for stability, keep live asset passes enabled, and prefer driver/OS upscaling over native high resolution.".to_string()
    } else if gpus.iter().any(|gpu| gpu.vendor == GpuVendor::Intel) {
        "Intel integrated GPU detected: start with --max-texture-size 1024 and lower to 768 if shared memory pressure remains high.".to_string()
    } else {
        "No low-end Intel iGPU profile detected; use Balanced texture preset defaults and adjust after testing.".to_string()
    }
}

pub fn parse_gpu_names(output: &str) -> Vec<GpuInfo> {
    output
        .lines()
        .filter_map(|line| {
            let name = line.trim();
            if name.is_empty() {
                return None;
            }
            let lower = name.to_ascii_lowercase();
            let vendor = if lower.contains("intel") {
                GpuVendor::Intel
            } else if lower.contains("nvidia")
                || lower.contains("geforce")
                || lower.contains("rtx")
                || lower.contains("gtx")
            {
                GpuVendor::Nvidia
            } else if lower.contains("amd") || lower.contains("radeon") {
                GpuVendor::Amd
            } else if lower.contains("apple") {
                GpuVendor::Apple
            } else {
                GpuVendor::Unknown
            };
            Some(GpuInfo {
                vendor,
                name: name.to_string(),
            })
        })
        .collect()
}

use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct IntelGpuState {
    pub card_path: PathBuf,
    pub original_min: u32,
    pub original_max: u32,
}

pub fn find_intel_gpu_card() -> Option<PathBuf> {
    let drm_dir = Path::new("/sys/class/drm");
    if !drm_dir.exists() {
        return None;
    }
    if let Ok(entries) = fs::read_dir(drm_dir) {
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name.starts_with("card") && !name.contains('-') {
                if path.join("gt_RP0_freq_mhz").exists() {
                    return Some(path);
                }
            }
        }
    }
    let fallback = PathBuf::from("/sys/class/drm/card0");
    if fallback.join("gt_RP0_freq_mhz").exists() {
        Some(fallback)
    } else {
        None
    }
}

pub fn pin_intel_gpu() -> Option<IntelGpuState> {
    let card_path = find_intel_gpu_card()?;
    
    let rp0_path = card_path.join("gt_RP0_freq_mhz");
    let min_path = card_path.join("gt_min_freq_mhz");
    let max_path = card_path.join("gt_max_freq_mhz");
    
    let rp0_val: u32 = fs::read_to_string(&rp0_path).ok()?.trim().parse().ok()?;
    let orig_min: u32 = fs::read_to_string(&min_path).ok()?.trim().parse().ok()?;
    let orig_max: u32 = fs::read_to_string(&max_path).ok()?.trim().parse().ok()?;
    
    println!("🚀 [GPU Optimizer] Intel iGPU hardware limit detected (RP0 = {} MHz). Original range: {}-{} MHz.", rp0_val, orig_min, orig_max);
    
    if let Err(e) = fs::write(&min_path, rp0_val.to_string()) {
        println!("⚠️  [GPU Optimizer] Failed to write to {}: {}. Root permissions or CAP_SYS_ADMIN may be required.", min_path.display(), e);
        return None;
    }
    if let Err(e) = fs::write(&max_path, rp0_val.to_string()) {
        println!("⚠️  [GPU Optimizer] Failed to write to {}: {}. Root permissions or CAP_SYS_ADMIN may be required.", max_path.display(), e);
        let _ = fs::write(&min_path, orig_min.to_string());
        return None;
    }
    
    println!("✅ [GPU Optimizer] Successfully pinned Intel iGPU clock speed at its absolute hardware maximum: {} MHz.", rp0_val);
    
    Some(IntelGpuState {
        card_path,
        original_min: orig_min,
        original_max: orig_max,
    })
}

pub fn restore_intel_gpu(state: &IntelGpuState) -> io::Result<()> {
    let min_path = state.card_path.join("gt_min_freq_mhz");
    let max_path = state.card_path.join("gt_max_freq_mhz");
    
    fs::write(&min_path, state.original_min.to_string())?;
    fs::write(&max_path, state.original_max.to_string())?;
    println!("🔄 [GPU Optimizer] Restored Intel iGPU frequency range to original defaults: {}-{} MHz.", state.original_min, state.original_max);
    Ok(())
}

fn run(command: &str, args: &[&str]) -> io::Result<String> {
    let output = Command::new(command).args(args).output()?;
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

trait Pipe: Sized {
    fn pipe<T>(self, f: impl FnOnce(Self) -> T) -> T {
        f(self)
    }
}
impl<T> Pipe for T {}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn detects_intel_hd_620() {
        let gpus = parse_gpu_names("Intel(R) HD Graphics 620\n");
        assert_eq!(gpus[0].vendor, GpuVendor::Intel);
        assert!(recommendation_for(&gpus).contains("1024"));
    }
}
