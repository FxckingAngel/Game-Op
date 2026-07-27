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
                "(command -v lspci >/dev/null && lspci | grep -Ei 'vga|3d|display') || true",
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
