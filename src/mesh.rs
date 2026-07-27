use std::{
    collections::HashMap,
    fs, io,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone)]
pub struct MeshPolicy {
    pub cache_dir: Option<String>,
    pub output_dir: Option<String>,
    pub dry_run: bool,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct MeshReport {
    pub scanned: usize,
    pub optimized: usize,
    pub copied: usize,
    pub skipped: usize,
}

pub struct MeshReducer {
    policy: MeshPolicy,
}

impl MeshReducer {
    pub fn new(policy: MeshPolicy) -> Self {
        Self { policy }
    }

    pub fn optimize_available_cache(&self) -> io::Result<MeshReport> {
        let Some(cache_dir) = &self.policy.cache_dir else {
            println!("Mesh reduction disabled: pass --mesh-cache PATH to process an external mesh cache.");
            return Ok(MeshReport::default());
        };
        let input_root = PathBuf::from(cache_dir);
        if !input_root.exists() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("mesh cache does not exist: {}", input_root.display()),
            ));
        }
        let output_root = self
            .policy
            .output_dir
            .as_ref()
            .map(PathBuf::from)
            .unwrap_or_else(|| input_root.with_extension("game-op-mesh-optimized"));
        let mut report = MeshReport::default();
        self.walk(&input_root, &input_root, &output_root, &mut report)?;
        println!(
            "Mesh cache complete: scanned={}, optimized={}, copied={}, skipped={}, output={}",
            report.scanned,
            report.optimized,
            report.copied,
            report.skipped,
            output_root.display()
        );
        Ok(report)
    }

    fn walk(
        &self,
        input_root: &Path,
        current: &Path,
        output_root: &Path,
        report: &mut MeshReport,
    ) -> io::Result<()> {
        for entry in fs::read_dir(current)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                self.walk(input_root, &path, output_root, report)?;
                continue;
            }
            report.scanned += 1;
            let relative = path.strip_prefix(input_root).unwrap_or(&path);
            let output = output_root.join(relative);
            if is_obj_file(&path) {
                match self.optimize_obj(&path, &output) {
                    Ok(true) => report.optimized += 1,
                    Ok(false) => report.copied += 1,
                    Err(error) => {
                        report.skipped += 1;
                        eprintln!("Skipping mesh {}: {error}", path.display());
                    }
                }
            } else {
                report.skipped += 1;
            }
        }
        Ok(())
    }

    fn optimize_obj(&self, input: &Path, output: &Path) -> io::Result<bool> {
        if output_is_fresh(input, output) {
            return Ok(false);
        }
        let source = fs::read_to_string(input)?;
        let optimized = compact_obj_positions(&source);
        let changed = optimized.len() < source.len();
        if self.policy.dry_run {
            if changed {
                println!(
                    "dry-run: would losslessly compact OBJ mesh {}",
                    input.display()
                );
            } else {
                println!("dry-run: would copy mesh {}", input.display());
            }
            return Ok(changed);
        }
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent)?;
        }
        if changed {
            fs::write(output, optimized)?;
        } else {
            fs::copy(input, output)?;
        }
        Ok(changed)
    }
}

fn output_is_fresh(input: &Path, output: &Path) -> bool {
    match (fs::metadata(input), fs::metadata(output)) {
        (Ok(input_meta), Ok(output_meta)) => {
            output_meta.modified().ok() >= input_meta.modified().ok()
        }
        _ => false,
    }
}

fn compact_obj_positions(source: &str) -> String {
    let mut positions: Vec<String> = Vec::new();
    let mut remap: HashMap<usize, usize> = HashMap::new();
    let mut unique: HashMap<String, usize> = HashMap::new();
    let mut output_lines: Vec<String> = Vec::new();
    let mut original_index = 0usize;

    for line in source.lines() {
        if let Some(rest) = line.strip_prefix("v ") {
            original_index += 1;
            let key = rest.split_whitespace().collect::<Vec<_>>().join(" ");
            let next = if let Some(existing) = unique.get(&key) {
                *existing
            } else {
                positions.push(format!("v {key}"));
                let index = positions.len();
                unique.insert(key, index);
                index
            };
            remap.insert(original_index, next);
        }
    }

    for position in &positions {
        output_lines.push(position.clone());
    }
    for line in source.lines() {
        if line.starts_with("v ") {
            continue;
        }
        if line.starts_with("f ") {
            output_lines.push(remap_face_positions(line, &remap));
        } else if !line.trim().is_empty() && !line.trim_start().starts_with('#') {
            output_lines.push(line.to_string());
        }
    }
    output_lines.join("\n") + "\n"
}

fn remap_face_positions(line: &str, remap: &HashMap<usize, usize>) -> String {
    let mut parts = line.split_whitespace();
    let mut remapped = vec![parts.next().unwrap_or("f").to_string()];
    for part in parts {
        let fields = part.split('/').collect::<Vec<_>>();
        let token = if let Ok(index) = fields[0].parse::<usize>() {
            if let Some(next) = remap.get(&index) {
                let mut next_fields = fields.clone();
                let next_position = next.to_string();
                next_fields[0] = &next_position;
                next_fields.join("/")
            } else {
                part.to_string()
            }
        } else {
            part.to_string()
        };
        remapped.push(token);
    }
    remapped.join(" ")
}

fn is_obj_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.eq_ignore_ascii_case("obj"))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compacts_duplicate_obj_positions_without_changing_faces() {
        let source = "# comment\nv 0 0 0\nv 1 0 0\nv 0 0 0\nf 1 2 3\n";
        let optimized = compact_obj_positions(source);
        assert!(optimized.contains("v 0 0 0\nv 1 0 0"));
        assert!(optimized.contains("f 1 2 1"));
        assert!(!optimized.contains("# comment"));
    }
}
