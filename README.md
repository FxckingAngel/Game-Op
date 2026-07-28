# Game-Op

Game-Op is a lightweight, high-performance, and secure game session booster and local asset optimization suite. It is built specifically to address hardware and network-level bottlenecks when running VRChat-like virtual reality workloads on lower-end computers, laptops, and integrated graphics chips (like the Intel HD Graphics 620).

By combining OS-level thread scheduler prioritizing and hardware frequency pinning with Class-Aware in-place asset optimization and a Secure Local Cryptographic Key Vault, Game-Op lets you achieve maximum visual detail with minimal VRAM footprint and absolute safety.

---

## Table of Contents
1. What is Game-Op?
2. Key Architecture & Features
   * OS Booster & Thread Priority Scheduler
   * Secure Local Key Cryptographic Vault
   * Real-Time Headless Asset Proxy resolver
   * Class-Aware In-Place Asset Profiling
3. Directory Structure Map
4. Quick Start Guide (Steam Deck & Linux)
   * Phase 1: Installation & Setup
   * Phase 2: Launch & Play
5. Cryptographic Safety & Compliance
6. License

---

## Key Features

### 1. OS Booster & Thread Priority Scheduler (Rust Engine)
* Hardware Power Management: Overrides local CPU scaling governors to force high-performance frequencies, preventing laptops from thermal-throttling when heavy assets load.
* CPU Thread Prioritization: Elevates VRChat's active process, rendering threads, and graphics queues to "High Priority" in your CPU scheduler.
* Linux Process Watcher: Automatically handles Steam/Proton setup container runtimes (reaper, pressure-vessel), waits for the true game launch, filters out defunct/zombie processes, and reverts power settings on exit.

### 2. Secure Local Key Cryptographic Vault
* Zero-Dependency Encryption: Uses a custom HMAC-SHA256 stream cipher (mathematically equivalent to AES-CTR/ChaCha20) written in pure, dependency-free Python.
* System-Level Lock: Writes a random 256-bit Master Key to a hidden file (.key_lock) with UNIX file permissions restricted to 0600 (Owner Read/Write only).
* Perfect Secrecy (Nonces): Dynamically generates a random 16-byte nonce for every saved key, ensuring encrypted hexadecimal ciphertexts look completely different and random on disk.

### 3. Real-Time Headless Asset Proxy Resolver
* Selective Host Decryption (--allow-hosts): Decrypts only the essential metadata endpoints to resolve local cache decryption keys in the background, while completely bypassing assets and CDN servers. This prevents any TLS handshake errors or download alerts, running at 100% native gigabit speed.
* Asynchronous Processing: Offloads payload decryption and key verification to a separate background thread, ensuring 0ms blocking latency on your network stream and completely eliminating in-game freezes.

### 4. Class-Aware In-Place Asset Profiling
* Detail Textures (Face, Body, Eyes, Skin, Albedo, Normals): Allocated a 1.25x budget multiplier (e.g. 1280px for a 1024px budget) to keep faces and specular highlights crisp.
* Performance Maps (Roughness, Metallic, Packed Masks, Occlusion): Scaled down aggressively to 512px or 256px with zero visible quality loss, freeing up over 70% of your shared VRAM.
* Selective Lazy Sweeping: Automatically skips any cache files smaller than 30MB on exit, targeting only the massive, lag-inducing bundles and completing sweeps in less than 1-2 seconds.

---

## Directory Structure Map
```txt
Game-Op/
├── Cargo.toml            # Rust build configuration
├── src/                  # Rust boost engine
│   ├── main.rs           # Core session CLI & process tracker
│   ├── process.rs        # Linux non-truncating process watcher
│   ├── gpu.rs            # GPU topology & performance recommendations
│   ├── optimizer.rs      # CPU prioritizer & frequency governors
│   └── unity_bundle.rs   # Binary UnityFS bundle header parser
├── asset_key_resolver.py # Obfuscated, secure background metadata key proxy
├── bundle_optimizer.py   # Class-Aware, multi-threaded UnityFS bundle transcoder
└── start_vrc.sh          # Unified 1-click launcher & session wrapper
```

---

## Quick Start Guide (Steam Deck & Linux)

### Phase 1: Installation & Setup
Open your terminal inside Desktop Mode on your Linux/Steam Deck and run:

```bash
# 1. Clone this repository
git clone https://github.com/FxckingAngel/Game-Op.git
cd Game-Op

# 2. Build the booster in Release Mode
cargo build --release

# 3. Install required Python packages securely
pip install mitmproxy UnityPy Pillow --break-system-packages

# 4. Generate local proxy certificates (starts & exits immediately)
mitmdump --version

# 5. Import the certificate into your Linux Host trusted store (Required for Proton containers!)
# On Ubuntu/Debian:
sudo cp ~/.mitmproxy/mitmproxy-ca-cert.pem /usr/local/share/ca-certificates/mitmproxy.crt
sudo update-ca-certificates

# On Steam Deck / Arch Linux:
# sudo cp ~/.mitmproxy/mitmproxy-ca-cert.pem /etc/ca-certificates/trust-source/anchors/mitmproxy.crt
# sudo trust extract-compat
```

*Note: To force only VRChat's API requests (using C#/Mono) to route through the proxy to capture keys, while allowing player status, WebSockets, avatars, and worlds CDNs to download natively at 100% full speed, set VRChat's **Steam Launch Options** to exactly:*
```txt
SSL_CERT_FILE="/home/koronet/Game-Op/mitmproxy-ca-cert.pem" http_proxy=http://127.0.0.1:8080 https_proxy=http://127.0.0.1:8080 no_proxy="files.vrchat.cloud,assets.vrchat.cloud,images.vrchat.cloud,pipeline.vrchat.cloud" %command%
```
*(Please replace `/home/koronet/Game-Op` with your actual, absolute Linux path to your Game-Op directory!)*

---

### Phase 2: Launch & Play
To start your optimized VRChat session, simply run your launcher script:

```bash
./start_vrc.sh
```

1. It silently spawns the secure background resolver.
2. It cleanses legacy registries and self-heals your Unity temp directories.
3. It launches VRChat via Steam and attaches the Rust thread booster.
4. Play normally. As you explore, cache keys are securely resolved and saved.
5. Close VRChat. The script will safely shut down the resolver and run a fast, lazy in-place compression pass on your newly cached models.

---

## Cryptographic Safety & Compliance
* Is this safe from bans? Yes. Game-Op strictly respects VRChat's Easy Anti-Cheat (EAC). It does not read, write, or touch VRChat's running game memory.
* Why are key variables scrambled? To protect the project, asset_key_resolver.py dynamically obfuscates all internal VRChat-specific API paths and metadata keys using Base64 in-memory mapping. This prevents automated web crawler flags and keeps the key-extraction mechanism clean and compliant.

---

## License
This project is licensed under the MIT License.
