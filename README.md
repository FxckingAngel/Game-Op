# Game-Op

Game-Op is a lightweight game-session optimizer for low-end PCs and laptops. It
makes heavy 3D and VR titles run smoother on weak hardware (for example an Intel
HD Graphics 620 iGPU) by combining OS-level performance tuning with in-place
optimization of the game's own downloaded assets. The current focus is VRChat on
Linux via Proton, but the asset optimizer works on any Unity game.

It operates only on local files your client already downloaded and on OS
scheduler and power settings. It does not read or modify game memory.

---

## Requirements

- Linux (Ubuntu/Kubuntu/Steam Deck and similar). Windows/macOS support is
  partial; see "Platform support" below.
- Rust toolchain (`cargo`) - the launcher builds the booster for you.
- Python 3.9+ with these packages:

  ```bash
  pip install UnityPy Pillow cryptography
  # optional, only for the key-capture proxy:
  pip install mitmproxy pycryptodome
  ```

- `inotify-tools` for instant live optimization (optional; falls back to a
  periodic sweep):

  ```bash
  sudo apt install inotify-tools
  ```

---

## Quick start

```bash
git clone https://github.com/FxckingAngel/Game-Op.git
cd Game-Op
./start_vrc.sh
```

That single command builds the Rust booster if needed, cleans up stale Proton
state, launches VRChat through Steam, optimizes assets live as they download,
and runs a final optimization sweep when you close the game.

One-time setup: set VRChat's Steam Launch Options (Steam > VRChat > Properties)
so the game picks up the performance environment. Replace the path with your
actual absolute path to this folder:

```txt
DXVK_CONFIG_FILE=/path/to/Game-Op/dxvk.conf DXVK_ASYNC=1 DXVK_FRAME_PACE=low-latency mesa_glthread=true MESA_NO_ERROR=1 INTEL_PRECISE_TRIG=0 %command%
```

## Enabling VRChat decryption (required to optimize VRChat bundles)

VRChat encrypts its cached bundles, so the optimizer cannot shrink them without
the per-file decryption keys. Game-Op captures those keys from VRChat's own API
as you load avatars and worlds, using a local proxy. Set it up once:

```bash
./setup_keycapture.sh
```

That installs mitmproxy if needed, generates a local certificate, and prints
the exact Steam Launch Options to paste (they replace the ones above and add
the proxy + certificate settings). After that, `./start_vrc.sh` starts the
key-capture proxy automatically; keys are saved to `~/Game-Op/vrc_keys.db` and
the optimizer decrypts and shrinks those bundles on the fly and on exit.

Note: this routes only `api.vrchat.cloud` through a local proxy to read the
keys your own client already receives; CDN/asset downloads stay direct. It
touches VRChat's API traffic, not game memory - be aware of VRChat's Terms of
Service when deciding to use it. To run without it (no VRChat decryption), set
`GAME_OP_KEYCAP=0 ./start_vrc.sh`.

---

## What it does

### OS booster (Rust engine)
- Detects your GPU and CPU across Windows, macOS, and Linux.
- On Linux, sets the CPU governor to performance and raises the game's render
  threads to high priority, then reverts on exit.
- On Intel Linux, can pin the iGPU frequency high (see `setup_gpu_permissions.sh`).

### Asset optimization
- Class-aware texture downscaling: detail textures (face, skin, albedo, normals)
  keep a larger budget; mask/roughness/metallic maps are scaled down more
  aggressively.
- GPU texture-format compression (optional): re-encodes textures into compact
  GPU formats (BC7, DXT1, or ASTC). VRAM use is `bytes_per_pixel(format) x width
  x height`, so this shrinks the per-pixel cost and rescues uncompressed
  textures - up to about 75-87% VRAM for those.
- Lossless mesh vertex compaction.
- Live optimization: assets are optimized the moment they finish downloading
  (via inotify), plus a full sweep on exit.

### Secure local key vault (VRChat)
- To decrypt VRChat's local cache, per-file keys are cached in `vrc_keys.db`,
  encrypted with AES-256-GCM.
- The encryption key is derived at runtime from your machine's hardware ID
  (PBKDF2-HMAC-SHA256) and is never written to disk. The database is created
  with owner-only (0600) permissions.

---

## Enabling GPU texture-format compression

This is experimental and off by default. Enable it for a run with:

```bash
GAME_OP_GPU_COMPRESS=1 ./start_vrc.sh
# optionally choose a format:
GAME_OP_GPU_COMPRESS=1 GAME_OP_GPU_FORMAT=bc7 ./start_vrc.sh
```

Formats: `auto` (opaque -> DXT1, alpha -> BC7), `bc7`, `dxt5`, `bc1`,
`astc4x4`, `astc5x5`, `astc6x6`, `astc8x8`.

---

## Using the optimizer on any Unity game

The bundle optimizer runs standalone on any Unity asset directory. Use
`--generic` to skip the VRChat-specific key pipeline:

```bash
python3 bundle_optimizer.py <input_dir> <output_dir> 1024 --generic --gpu-compress
```

Run `python3 bundle_optimizer.py --help` for all options.

---

## Measuring results

`validate_on_laptop.py` runs the optimizer on a safe copy of your cache (your
real cache is never modified) and reports real before/after asset sizes:

```bash
python3 validate_on_laptop.py --sample 20 --gpu-compress
```

---

## Platform support

- Portable today: CPU/power tuning, process and GPU detection, and the Unity
  asset optimizer (any OS, any Unity game).
- Linux-only today: GPU clock pinning (Intel sysfs), the `start_vrc.sh`
  launcher, and the permission/DXVK helpers. Windows/macOS launchers are not yet
  provided.

---

## Repository layout

- `src/` - Rust booster (process watcher, GPU/CPU tuning, asset staging).
- `bundle_optimizer.py` - Unity bundle texture/mesh optimizer.
- `asset_key_resolver.py` - background VRChat key-capture proxy and live transcoder.
- `start_vrc.sh` - one-command launcher and optimizer.
- `validate_on_laptop.py` - on-machine validation and measurement harness.
- `setup_gpu_permissions.sh` - udev rules for non-root Intel iGPU frequency control.
- `dxvk.conf` - tuned DXVK configuration.

---

## License

MIT. See the license header in the project.
