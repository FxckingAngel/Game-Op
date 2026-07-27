# Game-Op

Game-Op is a lightweight, cross-platform game performance session optimizer aimed at lower-end and integrated GPUs such as Intel HD Graphics 620. It watches for a target game such as VRChat, applies reversible OS-level performance settings for that session, and reverts them when the game exits.

Game-Op is designed to optimize aggressively without intentionally destroying visual quality: textures are resized only when they exceed the selected memory budget, aspect ratio is preserved, Lanczos filtering is used for high-quality downscaling, and mesh optimization is limited to lossless OBJ compaction. On an Intel Core i3-7100U with Intel HD Graphics 620, this targets the bottlenecks that usually hurt VRChat-like workloads: power throttling, scheduler priority, oversized textures, duplicate mesh data, and shared-memory/VRAM pressure.

## Current capabilities

- Detect a target process launch on Windows, macOS, and Linux.
- Switch OS power behavior for the session where native tools expose it.
- Set process priority through normal user-space scheduler controls.
- Provide guarded integration points for GPU vendor per-application profiles and driver/OS upscaling.
- Run live asset passes while the target game is open, so newly-added cache files can be optimized during the game session.
- Accept a single `--asset-cache` / `--asset-output` pair for VRChat-like avatar/world cache trees, then route supported files through specialized optimizers.
- Recursively build an optimized copy of texture assets, resizing PNG/JPEG/WebP files that exceed the selected maximum dimension.
- Recursively build an optimized copy of OBJ mesh assets by removing duplicate vertex-position records and comments without changing face geometry.
- Stage unsupported assets such as bundles, metadata, audio, shaders, and config files into the output tree unchanged so optimized caches stay complete.

## Usage

```bash
cargo run -- --target VRChat --dry-run --once
cargo run -- --target VRChat --asset-cache ./vrchat-cache --asset-output ./vrchat-cache-optimized --max-texture-size 1024 --live-asset-pass-seconds 15 --dry-run
cargo run -- --target VRChat --asset-cache ./vrchat-cache --asset-output ./vrchat-cache-optimized --max-texture-size 1024
```

Use `--dry-run` first. Some platform commands may require the same permissions that the operating system's own power or scheduler tools require.

## Intel HD Graphics 620 quality preset

For a 4-thread Intel Core i3-7100U with Intel HD Graphics 620, start with:

```bash
cargo run -- --target VRChat --asset-cache ./vrchat-cache --asset-output ./vrchat-cache-optimized --max-texture-size 1024 --live-asset-pass-seconds 15 --dry-run
```

If VRAM or shared memory pressure remains high, lower `--max-texture-size` to `768` or `512`. If the game is already stable and you prefer sharper close-up avatar detail, raise it to `1536` or `2048` only for caches that fit in memory.

## Real-time asset optimization boundary

The live pass is real-time in the safe, out-of-process sense: while the game is running, Game-Op periodically scans the opt-in cache paths for avatar/world loads and writes optimized replacements plus passthrough files to a separate complete output cache. It does not inject into VRChat, hook DirectX/OpenGL/Vulkan, patch Unity memory, or bypass platform validation. Those approaches are fragile and may violate game/platform rules.

For true in-engine replacement, VRChat or the game engine would need to expose a supported mod/plugin/cache API that allows the game to request optimized assets. Game-Op's implementation keeps that boundary clean so the optimizer can be integrated with an official API later.

## Texture and asset memory reduction policy

VRChat worlds and avatars are loaded and validated by VRChat/Unity. Real-time texture replacement inside a running game would generally require process injection, graphics API interception, cache tampering, or bypassing creator/platform expectations. Game-Op therefore uses a safer design:

1. Operate only on files the user explicitly points to with `--asset-cache`, `--texture-cache`, or `--mesh-cache`.
2. Build separate optimized caches instead of overwriting creator assets.
3. Preserve texture aspect ratio with high-quality Lanczos filtering for downscales.
4. Keep mesh reduction lossless unless a future supported in-engine LOD API allows visible-quality-aware simplification.
5. Avoid kernel drivers, anti-cheat bypasses, shader interception, or undocumented game memory access.

The implemented all-asset pipeline stages unsupported files unchanged, the texture pass currently supports PNG, JPEG, and WebP, and the mesh pass currently supports OBJ lossless compaction. DDS/KTX/Unity asset-bundle transcoding and quality-aware mesh LOD generation can be added later with dedicated decoders while keeping the same safety boundary.
