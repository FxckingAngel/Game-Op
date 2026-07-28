mod assets;
mod gpu;
mod mesh;
mod optimizer;
mod process;
mod profile;
mod texture;
mod unity_bundle;
mod vrchat;
mod watch;

use assets::{AssetStagePolicy, AssetStager};
use mesh::{MeshPolicy, MeshReducer};
use optimizer::{Optimizer, SessionConfig};
use process::ProcessWatcher;
use profile::ProfileDefaults;
use std::{env, path::Path, thread, time::Duration};
use texture::{TexturePolicy, TextureReducer};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_help();
        return;
    }

    let dry_run = args.iter().any(|arg| arg == "--dry-run");
    let once = args.iter().any(|arg| arg == "--once");
    let verbose = args.iter().any(|arg| arg == "--verbose" || arg == "-v");
    let run_game = args.iter().any(|arg| arg == "--run-game");
    let redirect_cache = args.iter().any(|arg| arg == "--redirect-cache");
    let revert_cache = args.iter().any(|arg| arg == "--revert-cache");
    let profile = ProfileDefaults::from_name(value_after(&args, "--profile").as_deref());
    let target = value_after(&args, "--target")
        .or(profile.target.clone())
        .unwrap_or_else(|| default_target().to_string());
    let asset_cache_dir = value_after(&args, "--asset-cache")
        .or_else(|| {
            let detected = auto_detect_vrchat_cache();
            if let Some(ref path) = detected {
                println!("Auto-detected VRChat cache directory: {}", path);
            }
            detected
        });
    let asset_output_dir = value_after(&args, "--asset-output");
    let cache_dir = value_after(&args, "--texture-cache").or_else(|| asset_cache_dir.clone());
    let output_dir = value_after(&args, "--texture-output").or_else(|| asset_output_dir.clone());
    let mesh_cache_dir = value_after(&args, "--mesh-cache").or_else(|| asset_cache_dir.clone());
    let mesh_output_dir = value_after(&args, "--mesh-output").or_else(|| asset_output_dir.clone());
    let live_asset_pass_seconds = value_after(&args, "--live-asset-pass-seconds")
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(profile.live_asset_pass_seconds);
    let quality_preset =
        value_after(&args, "--quality-preset").unwrap_or_else(|| profile.quality_preset.clone());
    let max_texture_size = value_after(&args, "--max-texture-size")
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(profile.max_texture_size);

    if revert_cache {
        let cache_path = asset_cache_dir.clone();
        if let Some(path) = cache_path {
            if let Err(e) = revert_cache_redirection(Path::new(&path)) {
                eprintln!("Failed to revert cache redirection: {}", e);
            }
        } else {
            eprintln!("Could not auto-detect VRChat cache directory. Please specify it using --asset-cache PATH");
        }
        return;
    }

    if redirect_cache {
        let cache_path = asset_cache_dir.clone();
        let output_path = asset_output_dir
            .as_ref()
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| {
                std::path::PathBuf::from(cache_path.as_deref().unwrap_or("")).with_extension("game-op-assets-optimized")
            });
        
        if let Some(path) = cache_path {
            if let Err(e) = setup_cache_redirection(Path::new(&path), &output_path) {
                eprintln!("Failed to setup cache redirection: {}", e);
            }
        } else {
            eprintln!("Could not auto-detect VRChat cache directory. Please specify it using --asset-cache PATH");
        }
        if !run_game {
            return;
        }
    }

    let mut spawned_process = None;
    if run_game {
        if let Some(dash_idx) = args.iter().position(|arg| arg == "--") {
            let game_args = &args[dash_idx + 1..];
            if !game_args.is_empty() {
                let program = &game_args[0];
                let program_args = &game_args[1..];
                println!("Launching game: {} with args: {:?}", program, program_args);
                let mut cmd = std::process::Command::new(program);
                cmd.args(program_args);
                if let Some(dir) = Path::new(program).parent() {
                    if !dir.as_os_str().is_empty() {
                        cmd.current_dir(dir);
                    }
                }
                match cmd.spawn() {
                    Ok(child) => {
                        spawned_process = Some(crate::process::GameProcess {
                            pid: child.id(),
                            name: target.clone(),
                        });
                    }
                    Err(e) => {
                        eprintln!("Failed to launch game: {}", e);
                        return;
                    }
                }
            } else {
                eprintln!("Error: --run-game specified but no command found after '--'");
                return;
            }
        } else {
            eprintln!("Error: --run-game specified but no '--' delimiter found");
            return;
        }
    }

    let config = SessionConfig {
        target_process: target.clone(),
        dry_run,
        set_process_priority: true,
        enable_power_mode: true,
        apply_gpu_profile: true,
        enable_driver_upscaling: true,
    };

    let mut optimizer = Optimizer::new(config);
    let watcher = ProcessWatcher::new(target.clone());
    let reducer = TextureReducer::new(TexturePolicy {
        cache_dir,
        output_dir,
        max_texture_size,
        quality_preset,
        dry_run,
        verbose,
    });
    let mesh_reducer = MeshReducer::new(MeshPolicy {
        cache_dir: mesh_cache_dir,
        output_dir: mesh_output_dir,
        dry_run,
        verbose,
    });
    let asset_stager = AssetStager::new(AssetStagePolicy {
        cache_dir: asset_cache_dir.clone(),
        output_dir: asset_output_dir,
        dry_run,
    });
    if let Some(asset_cache) = &asset_cache_dir {
        if let Ok(report) = vrchat::analyze_cache(Path::new(asset_cache), max_texture_size) {
            vrchat::print_recommendations(&report);
        }
    }

    let mut has_started = false;
    println!("Game-Op watching for {target} (dry_run={dry_run})");
    loop {
        let process_to_track = if let Some(ref proc) = spawned_process {
            if watcher.is_running(proc.pid).unwrap_or(false) {
                Some(proc.clone())
            } else {
                println!("Spawned game process exited; closing Game-Op");
                break;
            }
        } else {
            match watcher.find() {
                Ok(Some(proc)) => Some(proc),
                Ok(None) => None,
                Err(error) => {
                    eprintln!("Process scan failed: {error}");
                    if once && !has_started {
                        thread::sleep(Duration::from_secs(5));
                        continue;
                    } else if once {
                        break;
                    }
                    thread::sleep(Duration::from_secs(5));
                    continue;
                }
            }
        };

        match process_to_track {
            Some(process) => {
                has_started = true;
                println!("Detected {} with pid {}", process.name, process.pid);
                if let Err(error) = optimizer.apply(&process) {
                    eprintln!("Failed to apply optimizations: {error}");
                }
                run_asset_pass(&reducer, &mesh_reducer, &asset_stager);
                while watcher.is_running(process.pid).unwrap_or(false) {
                    thread::sleep(Duration::from_secs(live_asset_pass_seconds));
                    run_asset_pass(&reducer, &mesh_reducer, &asset_stager);
                }
                println!("{} closed; reverting session settings", process.name);
                if let Err(error) = optimizer.revert() {
                    eprintln!("Failed to revert optimizations: {error}");
                }
                if once || spawned_process.is_some() {
                    break;
                }
            }
            None => {
                if once && has_started {
                    break;
                }
                thread::sleep(Duration::from_secs(2));
            }
        }
    }
}

fn value_after(args: &[String], flag: &str) -> Option<String> {
    args.windows(2)
        .find(|window| window[0] == flag)
        .map(|window| window[1].clone())
}

fn default_target() -> &'static str {
    if cfg!(target_os = "windows") {
        "VRChat.exe"
    } else {
        "VRChat"
    }
}

fn print_help() {
    println!("Game-Op\n\nUsage: game-op [OPTIONS] [--run-game -- PROGRAM [ARGS...]]\n\nOptions:\n  --profile PROFILE              HQ-Low-End or Performance presets\n  --target PROCESS               The executable name to watch (default: VRChat)\n  --asset-cache PATH             Path to the raw/downloaded VRChat cache folder (auto-detects if omitted)\n  --asset-output PATH            Path to save optimized copies of assets\n  --verbose, -v                  Print active file optimizations in real-time\n  --redirect-cache               Rename active VRChat cache and symlink it to optimized outputs\n  --revert-cache                 Remove cache symlinks and restore original VRChat cache backups\n  --run-game -- PROGRAM ARGS     Steam Launcher wrapper. Spawns, monitors, and reverts when exited\n  --dry-run                      Simulation mode. No files or OS settings modified\n  --once                         Run a single optimization sweep and exit\n");
}

fn run_asset_pass(
    reducer: &TextureReducer,
    mesh_reducer: &MeshReducer,
    asset_stager: &AssetStager,
) {
    if !reducer.has_cache_dir() {
        return;
    }
    if let Err(error) = reducer.optimize_available_cache() {
        eprintln!("Texture cache optimization skipped: {error}");
    }
    if let Err(error) = mesh_reducer.optimize_available_cache() {
        eprintln!("Mesh cache optimization skipped: {error}");
    }
    if let Err(error) = asset_stager.stage_unoptimized_assets() {
        eprintln!("Asset staging skipped: {error}");
    }
}

fn auto_detect_vrchat_cache() -> Option<String> {
    if cfg!(target_os = "windows") {
        if let Ok(user_profile) = std::env::var("USERPROFILE") {
            let path = format!(r"{}\AppData\LocalLow\VRChat\VRChat\Cache-WindowsPlayer", user_profile);
            let backup_path = format!("{}.game-op-original", path);
            if Path::new(&path).exists() {
                return Some(path);
            } else if Path::new(&backup_path).exists() {
                return Some(path);
            }
        }
    } else if cfg!(target_os = "linux") || cfg!(target_os = "macos") {
        if let Ok(home) = std::env::var("HOME") {
            let candidates = [
                format!("{}/.steam/steam/steamapps/compatdata/438100/pfx/drive_c/users/steamuser/AppData/LocalLow/VRChat/VRChat/Cache-WindowsPlayer", home),
                format!("{}/.local/share/Steam/steamapps/compatdata/438100/pfx/drive_c/users/steamuser/AppData/LocalLow/VRChat/VRChat/Cache-WindowsPlayer", home),
                format!("{}/.var/app/com.valvesoftware.Steam/.local/share/Steam/steamapps/compatdata/438100/pfx/drive_c/users/steamuser/AppData/LocalLow/VRChat/VRChat/Cache-WindowsPlayer", home),
                "/run/media/mmcblk0p1/steamapps/compatdata/438100/pfx/drive_c/users/steamuser/AppData/LocalLow/VRChat/VRChat/Cache-WindowsPlayer".to_string(),
            ];
            for path in &candidates {
                let backup_path = format!("{}.game-op-original", path);
                if Path::new(path).exists() {
                    return Some(path.clone());
                } else if Path::new(&backup_path).exists() {
                    return Some(path.clone());
                }
            }
        }
    }
    None
}

fn setup_cache_redirection(real_cache: &Path, optimized_cache: &Path) -> std::io::Result<()> {
    let original_cache = real_cache.with_extension("game-op-original");

    if !real_cache.exists() && !original_cache.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Real cache directory not found at: {}", real_cache.display()),
        ));
    }
    
    if !optimized_cache.exists() {
        std::fs::create_dir_all(optimized_cache)?;
    }
    
    if let Ok(metadata) = std::fs::symlink_metadata(real_cache) {
        if metadata.file_type().is_symlink() {
            println!("Cache redirection is already active ({} is a symlink/junction)", real_cache.display());
            return Ok(());
        }
    }
    
    if !original_cache.exists() {
        std::fs::rename(real_cache, &original_cache)?;
        println!("Backed up original cache to {}", original_cache.display());
    } else {
        println!("Backup cache already exists at {}, skipping rename", original_cache.display());
    }
    
    let symlink_result = {
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(optimized_cache, real_cache)
        }
        #[cfg(windows)]
        {
            let status = std::process::Command::new("cmd")
                .args(&[
                    "/C",
                    "mklink",
                    "/J",
                    real_cache.to_str().unwrap_or_default(),
                    optimized_cache.to_str().unwrap_or_default(),
                ])
                .status();
            match status {
                Ok(s) if s.success() => Ok(()),
                _ => Err(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "Failed to create NTFS Directory Junction via mklink /J.",
                )),
            }
        }
    };

    if let Err(err) = symlink_result {
        if original_cache.exists() {
            let _ = std::fs::rename(&original_cache, real_cache);
        }
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to create redirect link (original cache restored): {}", err),
        ));
    }
    
    println!("Created redirection link: {} -> {}", real_cache.display(), optimized_cache.display());
    Ok(())
}

fn revert_cache_redirection(real_cache: &Path) -> std::io::Result<()> {
    let metadata = std::fs::symlink_metadata(real_cache);
    let is_symlink = match metadata {
        Ok(m) => m.file_type().is_symlink(),
        Err(_) => false,
    };
    
    if is_symlink {
        #[cfg(unix)]
        {
            std::fs::remove_file(real_cache)?;
        }
        #[cfg(windows)]
        {
            std::fs::remove_dir(real_cache)?;
        }
        println!("Removed redirection link {}", real_cache.display());
    }
    
    let original_cache = real_cache.with_extension("game-op-original");
    if original_cache.exists() {
        std::fs::rename(&original_cache, real_cache)?;
        println!("Restored original cache from {}", original_cache.display());
    } else {
        println!("No backup cache found at {}, nothing to restore", original_cache.display());
    }
    
    Ok(())
}
