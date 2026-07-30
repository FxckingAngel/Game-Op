#!/bin/bash
# ==================================================================
# Game-Op VRChat Ultimate Launcher & Optimizer (Zero-Touch Setup)
# ==================================================================
set -e

# Resolve paths dynamically
DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
cd "$DIR"

# Clean up any legacy, conflicting Cython .so or .c binaries, as well as stale compiled python bytecode and untracked bin files
# This completely prevents Python magic number conflicts, local file collisions, and AttributeErrors!
rm -f "$DIR"/*.so "$DIR"/*.c "$DIR"/*_bin.pyc "$DIR"/*_bin.py > /dev/null 2>&1 || true
rm -rf "$DIR"/__pycache__ > /dev/null 2>&1 || true

# Dynamically locate VRChat's Proton Wine prefix path globally on startup
WINEPREFIX_CANDIDATES=(
    "$HOME/.steam/steam/steamapps/compatdata/438100/pfx"
    "$HOME/.local/share/Steam/steamapps/compatdata/438100/pfx"
    "$HOME/.var/app/com.valvesoftware.Steam/.local/share/Steam/steamapps/compatdata/438100/pfx"
)
WINEPREFIX_PATH=""
for p_cand in "${WINEPREFIX_CANDIDATES[@]}"; do
    if [ -d "$p_cand" ]; then
        WINEPREFIX_PATH="$p_cand"
        break
    fi
done

# Automatically create a handy shortcut to VRChat's AppData/LocalLow folder directly in Game-Op!
if [ -n "$WINEPREFIX_PATH" ]; then
    VRC_LOCAL_LOW="$WINEPREFIX_PATH/drive_c/users/steamuser/AppData/LocalLow/VRChat/VRChat"
    if [ -d "$VRC_LOCAL_LOW" ]; then
        ln -sfn "$VRC_LOCAL_LOW" "$DIR/vrc-data"
    fi
fi

# Prepend ~/.local/bin to PATH to ensure user-installed binaries are found
export PATH="$HOME/.local/bin:$PATH"

# Bypass certificate pinning and SSL verification inside Unity's Mono/Proton framework
export MONO_TLS_ALLOW_UNTRUSTED=true

# ==================================================================
# Absolute Performance Limit Environment Variable Injectors (Mesa/DXVK)
# ==================================================================
# 1. Point DXVK to our high-performance tuned configuration file (prevents bash dot-syntax errors)
export DXVK_CONFIG_FILE="$DIR/dxvk.conf"

# 2. Enable asynchronous pipeline compile inside DXVK
export DXVK_ASYNC=1
export DXVK_FRAME_PACE=low-latency

# 3. Enable Mesa Multi-threaded OpenGL/Vulkan pipeline optimizations
export mesa_glthread=true
export MESA_GL_THREAD_CHANNEL=true

# 4. Disable driver-level error checking inside Mesa to reclaim valuable CPU clock cycles
export MESA_NO_ERROR=1

# 5. Optimize Intel-specific driver math calculations (prefers performance over double-precision trig)
export INTEL_PRECISE_TRIG=0

# Optional GPU texture-format compression (experimental). OFF by default; enable
# with GAME_OP_GPU_COMPRESS=1 ./start_vrc.sh  (optionally GAME_OP_GPU_FORMAT=bc7).
GPU_OPT_FLAGS=""
case "${GAME_OP_GPU_COMPRESS:-}" in
    1|true|yes|on)
        GPU_OPT_FLAGS="--gpu-compress"
        if [ -n "${GAME_OP_GPU_FORMAT:-}" ]; then
            GPU_OPT_FLAGS="$GPU_OPT_FLAGS --gpu-format ${GAME_OP_GPU_FORMAT}"
        fi
        echo "🎨 GPU texture-format compression ENABLED (experimental): $GPU_OPT_FLAGS"
        ;;
esac

# Ensure the high-performance Rust booster binary is compiled and up-to-date
if command -v cargo &> /dev/null; then
    echo "⚙️ Verifying and compiling high-performance Rust booster..."
    cargo build --release > /dev/null 2>&1 || true
fi

echo "=================================================================="
echo " 🚀 Starting Game-Op Ultimate VRChat Session..."
echo "=================================================================="

# Check if mitmdump is available (optional debug tool)
if ! command -v mitmdump &> /dev/null; then
    echo "💡 Note: mitmdump is not in your PATH. Debug proxy mode is unavailable (not required for native gameplay optimization)."
fi

# 1. Force kill any process holding port 8080 to guarantee it is free
PROXY_PORT=8080
echo "🧹 Ensuring port $PROXY_PORT is free..."
if command -v fuser &> /dev/null; then
    fuser -k $PROXY_PORT/tcp > /dev/null 2>&1 || true
fi
if command -v lsof &> /dev/null; then
    lsof -t -i :$PROXY_PORT | xargs kill -9 > /dev/null 2>&1 || true
fi

# 2. Clean up any leftover background proxies and hung game/crash handler processes gracefully
pkill -f mitmdump || true
pkill -f mitmproxy || true
echo "🧹 Safely cleaning up VRChat, Crash Handler, and Wine zombie processes..."
pkill -f VRChat || true
pkill -f UnityCrashHandler64 || true
pkill -f start_protected_game || true
pkill -f wineserver || true
pkill -f explorer.exe || true
pkill -f services.exe || true
pkill -f winedevice.exe || true
pkill -f plugplay.exe || true
pkill -f svchost.exe || true
pkill -f winedbg || true
if command -v wineserver &> /dev/null; then
    wineserver -k > /dev/null 2>&1 || true
fi
sleep 0.5

# 3. Cleanse any stale Steam lock files ONLY if Steam is not currently running
# This prevents startup locks without force-closing Steam if you already have it open!
if ! pgrep -x "steam" > /dev/null; then
    echo "🧹 Steam is not running. Cleansing stale Steam lock files..."
    rm -f "$HOME/.steam/steam.pid" > /dev/null 2>&1 || true
    rm -f "$HOME/.steam/steam/steam.pid" > /dev/null 2>&1 || true
    rm -f "$HOME/.local/share/Steam/steam.pid" > /dev/null 2>&1 || true
fi

# 4. Cleanse any legacy persistent global Wine proxy settings
# This ensures that standard HTTP/HTTPS CDN and image loaders bypass the proxy and run at native gigabit speed!
if [ -n "$WINEPREFIX_PATH" ]; then
    export WINEPREFIX="$WINEPREFIX_PATH"
    echo "🌐 Cleansing legacy global Wine proxy registries directly in user.reg..."
    # Disable global proxy in Wine prefix user.reg text file directly
    # This guarantees the proxy is disabled even if host has no wine or wine command fails
    python3 -c "
import os, re
reg_path = os.path.join(os.environ['WINEPREFIX'], 'user.reg')
if os.path.exists(reg_path):
    with open(reg_path, 'r', encoding='utf-8', errors='ignore') as f:
        lines = f.readlines()
    in_section = False
    new_lines = []
    for line in lines:
        if line.strip().startswith('[Software\\\\Microsoft\\\\Windows\\\\CurrentVersion\\\\Internet Settings]'):
            in_section = True
            new_lines.append(line)
            continue
        if in_section:
            if line.strip().startswith('['):
                in_section = False
            else:
                if line.startswith('\"ProxyEnable\"=') or line.startswith('\"ProxyServer\"='):
                    continue
        new_lines.append(line)
    final_lines = []
    for line in new_lines:
        final_lines.append(line)
        if line.strip().startswith('[Software\\\\Microsoft\\\\Windows\\\\CurrentVersion\\\\Internet Settings]'):
            final_lines.append('\"ProxyEnable\"=dword:00000000\n')
            final_lines.append('\"ProxyServer\"=\"\"\n')
    with open(reg_path, 'w', encoding='utf-8') as f:
        f.writelines(final_lines)
    print('✅ Proton registry user.reg cleansed of legacy proxies successfully!')
"

    # Self-healing fix for Unity Cache Temp directory creation errors
    python3 -c "
import os, shutil
local_low = os.path.join(os.environ['WINEPREFIX'], 'drive_c/users/steamuser/AppData/LocalLow')
if os.path.exists(local_low):
    print('🛠️ [Self-Healing] Checking and repairing AppData LocalLow directories...')
    
    # Clean up and consolidate any legacy directories/symlinks inside VRChat/VRChat
    vrc_dir = os.path.join(local_low, 'VRChat/VRChat')
    if os.path.exists(vrc_dir) and os.path.isdir(vrc_dir):
        cache_path = os.path.join(vrc_dir, 'Cache-WindowsPlayer')
        if os.path.islink(cache_path):
            print(f'  🧹 Unlinking legacy symlink: {cache_path}')
            os.unlink(cache_path)
        if os.path.exists(cache_path) and not os.path.isdir(cache_path):
            os.remove(cache_path)
        os.makedirs(cache_path, exist_ok=True)

        # Safely remove all un-needed legacy/exposed optimized directories to keep it exactly how VRChat does naturally
        for suffix in ['game-op-original', 'game-op-optimized', 'game-op-assets-optimized', 'game-op-mesh-optimized']:
            legacy_path = cache_path + '.' + suffix
            if os.path.exists(legacy_path) and os.path.isdir(legacy_path):
                print(f'  🧹 [Self-Healing] Removing legacy folder: {legacy_path}')
                shutil.rmtree(legacy_path)

        # Safely remove legacy Cache-WindowsPlayer-optimized (with dash) if it exists
        dash_legacy = os.path.join(vrc_dir, 'Cache-WindowsPlayer-optimized')
        if os.path.exists(dash_legacy) and os.path.isdir(dash_legacy):
            print(f'  🧹 [Self-Healing] Removing legacy folder: {dash_legacy}')
            shutil.rmtree(dash_legacy)

    # Self-heal and recreate healthy system directories
    for sub in ['Unity', 'VRChat', 'Unity/Temp', 'VRChat/VRChat']:
        path = os.path.join(local_low, sub)
        if os.path.islink(path):
            print(f'  🧹 Removing broken symlink: {path}')
            os.unlink(path)
        elif os.path.exists(path) and not os.path.isdir(path):
            os.remove(path)
        if not os.path.exists(path):
            os.makedirs(path, exist_ok=True)
            print(f'  ✅ Created healthy directory: {path}')
            
    # Correct permissions recursively
    for root, dirs, files in os.walk(local_low):
        for d in dirs:
            try:
                os.chmod(os.path.join(root, d), 0o755)
            except Exception:
                pass
        for f in files:
            try:
                os.chmod(os.path.join(root, f), 0o644)
            except Exception:
                pass

    # Clean up old massive VRChat output log files to free up disk space and reduce IO load
    vrc_log_dir = os.path.join(local_low, 'VRChat/VRChat')
    if os.path.exists(vrc_log_dir) and os.path.isdir(vrc_log_dir):
        import time
        deleted_logs_count = 0
        freed_bytes = 0
        now = time.time()
        for f in os.listdir(vrc_log_dir):
            if f.startswith('output_log_') and f.endswith('.txt'):
                fp = os.path.join(vrc_log_dir, f)
                try:
                    # Remove files older than 3 days
                    if os.path.isfile(fp) and (now - os.path.getmtime(fp)) > (3 * 86400):
                        freed_bytes += os.path.getsize(fp)
                        os.remove(fp)
                        deleted_logs_count += 1
                except Exception:
                    pass
        if deleted_logs_count > 0:
            print(f'  🧹 [Self-Healing] Cleansed {deleted_logs_count} old VRChat output logs, freeing up {freed_bytes / (1024*1024):.2f} MB!')

    print('✅ AppData LocalLow folders self-healed and permissions restored successfully!')
"
fi

# Ensure any legacy cache redirection symlinks are reverted to a normal folder to prevent game crashes
if [ -f "./target/release/game-op" ]; then
    echo "🧹 Restoring cache folder to native state to prevent Unity crashes..."
    ./target/release/game-op --revert-cache > /dev/null 2>&1 || true
fi

# ==================================================================
# REAL-TIME LIVE IN-PLACE OPTIMIZER SCHEDULER
# ==================================================================
CACHE_PATH="$HOME/.steam/steam/steamapps/compatdata/438100/pfx/drive_c/users/steamuser/AppData/LocalLow/VRChat/VRChat/Cache-WindowsPlayer"
HTTP_CACHE_PATH="$(dirname "$CACHE_PATH")/HTTPCache-WindowsPlayer"
TEXTURE_CACHE_PATH="$(dirname "$CACHE_PATH")/TextureDiskCache-WindowsPlayer"

# Guarantee all cache directories exist so monitoring watches do not fail
mkdir -p "$CACHE_PATH" "$HTTP_CACHE_PATH" "$TEXTURE_CACHE_PATH"

if command -v inotifywait &> /dev/null; then
    echo "⚡ [Kernel Link] Linux inotifywait detected! Enabling instant real-time live asset optimization..."
    (
        # Monitor the cache folder, HTTP cache, and Texture cache recursively for closed-after-write events
        inotifywait -m -r -e close_write --format "%w%f" "$CACHE_PATH" "$HTTP_CACHE_PATH" "$TEXTURE_CACHE_PATH" 2>/dev/null | while read -r filepath; do
            if [[ "$filepath" == *"__data" ]]; then
                # Instantly transcode the completed encrypted bundle in-place before the game loads it!
                python3 bundle_optimizer.py "$filepath" "$filepath" 1024 $GPU_OPT_FLAGS >/dev/null 2>&1 || true
            elif [[ "$filepath" == *"/HTTPCache-WindowsPlayer/"* ]]; then
                # Instantly transcode downloaded extensionless HTTP raw web texture in-place
                ./target/release/game-op --profile vrchat-hq-low-end --asset-cache "$filepath" --asset-output "$filepath" --once >/dev/null 2>&1 || true
            elif [[ "$filepath" == *"/TextureDiskCache-WindowsPlayer/"* ]]; then
                # Instantly transcode downloaded extensionless GPU disk texture in-place
                ./target/release/game-op --profile vrchat-hq-low-end --asset-cache "$filepath" --asset-output "$filepath" --once >/dev/null 2>&1 || true
            fi
        done
    ) &
    LIVE_OPT_PID=$!
else
    echo "💡 Notice: Install 'inotify-tools' (sudo apt install inotify-tools) to enable instant kernel-level real-time asset optimization!"
    echo "🔄 Falling back to high-performance 15-second periodic live sweeper..."
    (
        while true; do
            sleep 15
            # 1. Live-optimize Unity asset bundles inside Cache-WindowsPlayer
            if [ -d "$CACHE_PATH" ]; then
                python3 bundle_optimizer.py "$CACHE_PATH" "$CACHE_PATH" 1024 $GPU_OPT_FLAGS >/dev/null 2>&1 || true
            fi
            # 2. Live-optimize unencrypted HTTP textures inside HTTPCache-WindowsPlayer
            if [ -d "$HTTP_CACHE_PATH" ]; then
                ./target/release/game-op --profile vrchat-hq-low-end --asset-cache "$HTTP_CACHE_PATH" --asset-output "$HTTP_CACHE_PATH" --once >/dev/null 2>&1 || true
            fi
            # 3. Live-optimize unencrypted GPU textures inside TextureDiskCache-WindowsPlayer
            if [ -d "$TEXTURE_CACHE_PATH" ]; then
                ./target/release/game-op --profile vrchat-hq-low-end --asset-cache "$TEXTURE_CACHE_PATH" --asset-output "$TEXTURE_CACHE_PATH" --once >/dev/null 2>&1 || true
            fi
        done
    ) &
    LIVE_OPT_PID=$!
fi

# Ensure the background tasks turn off cleanly when this script exits/interrupts
cleanup() {
    echo ""
    echo "=================================================================="
    echo " 🧹 Shutting down session..."
    echo "=================================================================="
    if [ -n "$LIVE_OPT_PID" ]; then
        echo "Stopping background live bundle optimizer (PID $LIVE_OPT_PID)..."
        kill "$LIVE_OPT_PID" > /dev/null 2>&1 || true
    fi
}
trap cleanup EXIT

sleep 1.0

# 5. Launch Steam / VRChat safely (prevents deadlocks and avoids restarting Steam if already running)
if pgrep -x "steam" > /dev/null; then
    echo "🎮 Steam is already running! Sending launch command for VRChat..."
    echo "⚠️  [Warning] Steam is running in the background."
    echo "   To ensure keys are captured successfully, you MUST set VRChat's Steam Launch Options in the Steam GUI to:"
    echo "   SSL_CERT_FILE=\"$DIR/mitmproxy-ca-cert.pem\" http_proxy=http://127.0.0.1:8080 https_proxy=http://127.0.0.1:8080 no_proxy=\"files.vrchat.cloud,assets.vrchat.cloud,images.vrchat.cloud,pipeline.vrchat.cloud\" %command%"
    echo ""
    steam steam://rungameid/438100 > /dev/null 2>&1 &
else
    echo "🎮 Launching Steam..."
    echo "👉 NOTE: Please ensure your VRChat Steam Launch Options are set to exactly:"
    echo "   DXVK_CONFIG_FILE=$DIR/dxvk.conf DXVK_ASYNC=1 DXVK_FRAME_PACE=low-latency mesa_glthread=true MESA_GL_THREAD_CHANNEL=true MESA_NO_ERROR=1 INTEL_PRECISE_TRIG=0 %command%"
    echo ""

    (
        # Unset all game-specific performance variables to guarantee Steam client stability and prevent Chromium/steamwebhelper crashes!
        unset DXVK_CONFIG_FILE
        unset DXVK_ASYNC
        unset DXVK_FRAME_PACE
        unset mesa_glthread
        unset MESA_GL_THREAD_CHANNEL
        unset MESA_NO_ERROR
        unset INTEL_PRECISE_TRIG
        
        steam steam://rungameid/438100 > /dev/null 2>&1 &
    )
fi

# 6. Launch the Rust thread booster (blocks until VRChat exits)
# We configure the booster to perform in-place optimizations directly inside VRChat's official folders!
echo "⚡ Starting Game-Op OS booster & process priority tracker..."
./target/release/game-op --profile vrchat-hq-low-end --asset-cache "$CACHE_PATH" --asset-output "$CACHE_PATH" --verbose --once

echo "✅ VRChat has closed!"

# 7. Run the comprehensive in-place asset bundle, texture, and mesh optimizations on exit
echo ""
echo "=================================================================="
echo " ⚙️ Deep Bundle & Texture Optimization Sweep..."
echo "=================================================================="

# 1. Optimize raw Unity asset bundles in-place
python3 bundle_optimizer.py "$CACHE_PATH" "$CACHE_PATH" 1024 $GPU_OPT_FLAGS

# 2. Optimize meshes and textures in-place inside Cache-WindowsPlayer
echo "  Optimizing meshes and textures in-place inside Cache-WindowsPlayer..."
./target/release/game-op --profile vrchat-hq-low-end --asset-cache "$CACHE_PATH" --asset-output "$CACHE_PATH" --verbose --once

# 3. Optimize extensionless web textures in-place inside HTTPCache-WindowsPlayer
if [ -d "$HTTP_CACHE_PATH" ]; then
    echo "  Optimizing extensionless textures in-place inside HTTPCache-WindowsPlayer..."
    ./target/release/game-op --profile vrchat-hq-low-end --asset-cache "$HTTP_CACHE_PATH" --asset-output "$HTTP_CACHE_PATH" --verbose --once
fi

# 4. Optimize extensionless GPU textures in-place inside TextureDiskCache-WindowsPlayer
if [ -d "$TEXTURE_CACHE_PATH" ]; then
    echo "  Optimizing extensionless textures in-place inside TextureDiskCache-WindowsPlayer..."
    ./target/release/game-op --profile vrchat-hq-low-end --asset-cache "$TEXTURE_CACHE_PATH" --asset-output "$TEXTURE_CACHE_PATH" --once
fi

echo "=================================================================="
echo " 🎉 Optimization Sweep Completed! Ready for your next session."
echo "=================================================================="
