use image::{imageops::FilterType, DynamicImage, GenericImageView, ImageFormat};
use std::{
    fs, io,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone)]
pub struct TexturePolicy {
    pub cache_dir: Option<String>,
    pub output_dir: Option<String>,
    pub max_texture_size: u32,
    pub quality_preset: String,
    pub dry_run: bool,
    pub verbose: bool,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct TextureReport {
    pub scanned: usize,
    pub optimized: usize,
    pub copied: usize,
    pub skipped: usize,
}

#[derive(Clone)]
pub struct TextureReducer {
    policy: TexturePolicy,
}

impl TextureReducer {
    pub fn new(policy: TexturePolicy) -> Self {
        Self { policy }
    }

    pub fn has_cache_dir(&self) -> bool {
        self.policy.cache_dir.is_some()
    }

    pub fn optimize_available_cache(&self) -> io::Result<TextureReport> {
        let Some(cache_dir) = &self.policy.cache_dir else {
            println!("Texture reduction disabled: pass --texture-cache PATH to process an external asset cache.");
            return Ok(TextureReport::default());
        };

        let input_root = PathBuf::from(cache_dir);
        if !input_root.exists() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("texture cache does not exist: {}", input_root.display()),
            ));
        }

        let output_root = self
            .policy
            .output_dir
            .as_ref()
            .map(PathBuf::from)
            .unwrap_or_else(|| input_root.with_extension("game-op-optimized"));

        let mut report = TextureReport::default();
        let mut tasks = Vec::new();

        self.walk_and_collect(&input_root, &input_root, &output_root, &mut report, &mut tasks)?;

        if !tasks.is_empty() {
            use std::sync::{Arc, Mutex};
            use std::thread;

            let tasks = Arc::new(Mutex::new(tasks));
            let num_threads = std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(4)
                .min(8);

            let mut workers = Vec::new();
            for _ in 0..num_threads {
                let tasks = Arc::clone(&tasks);
                let reducer = self.clone();
                let handle = thread::spawn(move || {
                    let mut local_report = TextureReport::default();
                    loop {
                        let task = {
                            let mut guard = tasks.lock().unwrap();
                            guard.pop()
                        };
                        let Some((input, output)) = task else {
                            break;
                        };
                        match reducer.optimize_texture(&input, &output) {
                            Ok(true) => local_report.optimized += 1,
                            Ok(false) => local_report.copied += 1,
                            Err(error) => {
                                local_report.skipped += 1;
                                eprintln!("Skipping {}: {error}", input.display());
                            }
                        }
                    }
                    local_report
                });
                workers.push(handle);
            }

            for handle in workers {
                if let Ok(local_report) = handle.join() {
                    report.optimized += local_report.optimized;
                    report.copied += local_report.copied;
                    report.skipped += local_report.skipped;
                }
            }
        }

        if self.policy.verbose || report.optimized > 0 {
            println!(
                "Texture cache complete: scanned={}, optimized={}, copied={}, skipped={}, output={}",
                report.scanned,
                report.optimized,
                report.copied,
                report.skipped,
                output_root.display()
            );
        }
        Ok(report)
    }

    fn walk_and_collect(
        &self,
        input_root: &Path,
        current: &Path,
        output_root: &Path,
        report: &mut TextureReport,
        tasks: &mut Vec<(PathBuf, PathBuf)>,
    ) -> io::Result<()> {
        for entry in fs::read_dir(current)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                self.walk_and_collect(input_root, &path, output_root, report, tasks)?;
                continue;
            }

            report.scanned += 1;
            let relative = path.strip_prefix(input_root).unwrap_or(&path);
            let output = output_root.join(relative);
            if is_texture_file(&path) {
                tasks.push((path, output));
            } else {
                report.skipped += 1;
            }
        }
        Ok(())
    }

    fn optimize_texture(&self, input: &Path, output: &Path) -> io::Result<bool> {
        if output_is_fresh(input, output) && input != output {
            return Ok(false);
        }
        let reader = image::ImageReader::open(input)?
            .with_guessed_format()?;
        let format = reader.format().unwrap_or(image::ImageFormat::Png);
        let image = reader.decode().map_err(image_error)?;
        let (width, height) = image.dimensions();
        let max_size = max_size_for_texture(
            input,
            self.policy.max_texture_size,
            &self.policy.quality_preset,
        );
        let largest_side = width.max(height);
        let needs_resize = largest_side > max_size;

        if self.policy.dry_run {
            if needs_resize {
                println!(
                    "dry-run: would resize {} from {}x{} to max {}px",
                    input.display(),
                    width,
                    height,
                    max_size
                );
            } else {
                println!(
                    "dry-run: would copy already-budgeted texture {}",
                    input.display()
                );
            }
            return Ok(needs_resize);
        } else if self.policy.verbose {
            if needs_resize {
                println!(
                    "Optimizing texture {}: resizing from {}x{} to max {}px",
                    input.display(),
                    width,
                    height,
                    max_size
                );
            } else {
                if input != output {
                    println!(
                        "Copying texture {}: already-budgeted ({}x{})",
                        input.display(),
                        width,
                        height
                    );
                }
            }
        }

        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent)?;
        }

        if needs_resize {
            let resized = resize_preserving_aspect(image, max_size);
            save_image_with_format(&resized, output, format)?;
            Ok(true)
        } else {
            if input != output {
                fs::copy(input, output)?;
            }
            Ok(false)
        }
    }
}

fn max_size_for_texture(path: &Path, base: u32, preset: &str) -> u32 {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let multiplier = match preset {
        "performance" => 0.5,
        "balanced" => 0.75,
        "high-quality-low-end" => 1.0,
        "ultra-safe" => 1.5,
        _ => 1.0,
    };
    let class_multiplier = if contains_any(
        &name,
        &[
            "face",
            "head",
            "eye",
            "skin",
            "body",
            "hair",
            "cloth",
            "albedo",
            "diffuse",
            "basecolor",
        ],
    ) {
        1.25
    } else if contains_any(&name, &["normal", "_nrm", "bump"]) {
        1.25
    } else if contains_any(
        &name,
        &[
            "mask",
            "rough",
            "roughness",
            "metal",
            "metallic",
            "ao",
            "occlusion",
            "packed",
        ],
    ) {
        0.5
    } else if contains_any(&name, &["ui", "icon", "font"]) {
        1.0
    } else {
        1.0
    };
    ((base as f32 * multiplier * class_multiplier).round() as u32).max(256)
}

fn contains_any(name: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| name.contains(needle))
}

fn resize_preserving_aspect(image: DynamicImage, max_side: u32) -> DynamicImage {
    let (width, height) = image.dimensions();
    let scale = max_side as f32 / width.max(height) as f32;
    let next_width = (width as f32 * scale).round().max(1.0) as u32;
    let next_height = (height as f32 * scale).round().max(1.0) as u32;
    image.resize(next_width, next_height, FilterType::Lanczos3)
}

fn save_image_with_format(image: &DynamicImage, output: &Path, format: ImageFormat) -> io::Result<()> {
    image.save_with_format(output, format).map_err(image_error)
}

fn is_texture_file(path: &Path) -> bool {
    // 1. Check extension first (fast path)
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        if matches!(
            ext.to_ascii_lowercase().as_str(),
            "png" | "jpg" | "jpeg" | "webp"
        ) {
            return true;
        }
    }

    // 2. Fallback to magic bytes signature analysis for extensionless cached files!
    if let Ok(mut file) = fs::File::open(path) {
        let mut buffer = [0u8; 12];
        use std::io::Read;
        if file.read_exact(&mut buffer).is_ok() {
            // Check PNG magic signature: \x89PNG\r\n\x1a\n
            if buffer[0..4] == [0x89, 0x50, 0x4E, 0x47] {
                return true;
            }
            // Check JPEG magic signature: \xFF\xD8\xFF
            if buffer[0..3] == [0xFF, 0xD8, 0xFF] {
                return true;
            }
            // Check WEBP magic signature: RIFF....WEBP
            if &buffer[0..4] == b"RIFF" && &buffer[8..12] == b"WEBP" {
                return true;
            }
        }
    }
    false
}

fn output_is_fresh(input: &Path, output: &Path) -> bool {
    match (fs::metadata(input), fs::metadata(output)) {
        (Ok(input_meta), Ok(output_meta)) => {
            output_meta.modified().ok() >= input_meta.modified().ok()
        }
        _ => false,
    }
}

fn image_error(error: image::ImageError) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, error)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgba};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn downscales_large_texture_to_budget() {
        let root = temp_path("game-op-texture-test");
        let input = root.join("cache");
        let output = root.join("optimized");
        fs::create_dir_all(&input).unwrap();
        let source = input.join("avatar.png");
        let image = ImageBuffer::from_pixel(2048, 1024, Rgba([120u8, 80, 220, 255]));
        image.save(&source).unwrap();

        let reducer = TextureReducer::new(TexturePolicy {
            cache_dir: Some(input.to_string_lossy().into_owned()),
            output_dir: Some(output.to_string_lossy().into_owned()),
            max_texture_size: 512,
            quality_preset: "high-quality-low-end".to_string(),
            dry_run: false,
            verbose: false,
        });

        let report = reducer.optimize_available_cache().unwrap();
        let optimized = image::open(output.join("avatar.png")).unwrap();
        assert_eq!(report.optimized, 1);
        assert_eq!(optimized.dimensions(), (512, 256));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn applies_quality_preset_by_texture_class() {
        assert_eq!(
            max_size_for_texture(Path::new("avatar_face.png"), 1024, "high-quality-low-end"),
            1280
        );
        assert_eq!(
            max_size_for_texture(Path::new("metal_mask.png"), 1024, "high-quality-low-end"),
            512
        );
        assert_eq!(
            max_size_for_texture(Path::new("world.png"), 1024, "performance"),
            512
        );
    }

    fn temp_path(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("{name}-{nanos}"))
    }
}
