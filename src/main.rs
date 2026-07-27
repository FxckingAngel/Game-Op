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
    let profile = ProfileDefaults::from_name(value_after(&args, "--profile").as_deref());
    let target = value_after(&args, "--target")
        .or(profile.target.clone())
        .unwrap_or_else(|| default_target().to_string());
    let asset_cache_dir = value_after(&args, "--asset-cache");
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
    });
    let mesh_reducer = MeshReducer::new(MeshPolicy {
        cache_dir: mesh_cache_dir,
        output_dir: mesh_output_dir,
        dry_run,
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

    println!("Game-Op watching for {target} (dry_run={dry_run})");
    loop {
        match watcher.find() {
            Ok(Some(process)) => {
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
                if once {
                    break;
                }
            }
            Ok(None) => {
                if once {
                    println!("Target process not running");
                    break;
                }
                thread::sleep(Duration::from_secs(2));
            }
            Err(error) => {
                eprintln!("Process scan failed: {error}");
                if once {
                    break;
                }
                thread::sleep(Duration::from_secs(5));
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
    println!("Game-Op\n\nUsage: game-op [--profile PROFILE] [--target PROCESS] [--asset-cache PATH] [--asset-output PATH] [--texture-cache PATH] [--texture-output PATH] [--mesh-cache PATH] [--mesh-output PATH] [--quality-preset PRESET] [--max-texture-size PX] [--live-asset-pass-seconds SECONDS] [--dry-run] [--once]\n\nWatches for a game process, applies reversible OS/driver-level performance settings, and optionally prepares a creator-safe texture-cache reduction pass outside the game process.");
}

fn run_asset_pass(
    reducer: &TextureReducer,
    mesh_reducer: &MeshReducer,
    asset_stager: &AssetStager,
) {
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
