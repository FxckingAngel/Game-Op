#!/bin/bash
# ==================================================================
# Game-Op VRChat Ultimate Launcher & Optimizer (Zero-Touch Setup)
# ==================================================================
set -e

# Resolve paths dynamically
DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
cd "$DIR"

# Prepend ~/.local/bin to PATH to ensure user-installed mitmdump is found
export PATH="$HOME/.local/bin:$PATH"

# Bypass certificate pinning and SSL verification inside Unity's Mono/Proton framework
export MONO_TLS_ALLOW_UNTRUSTED=true

# Ensure secure black-box binaries are compiled locally
if [ -f "asset_key_resolver_bin.py" ] || [ -f "bundle_optimizer_bin.py" ]; then
    echo "🔒 Secure black-box python source detected."
    echo "⚙️ Automatically compiling and locking native binaries for your hardware..."
    chmod +x compile_binaries.sh
    ./compile_binaries.sh
fi

echo "=================================================================="
echo " 🚀 Starting Game-Op Ultimate VRChat Session..."
echo "=================================================================="

# Check if mitmdump is available
if ! command -v mitmdump &> /dev/null; then
    echo "Error: mitmdump is not in your PATH."
    echo "Please ensure it is installed by running: pip install mitmproxy --break-system-packages"
    exit 1
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

# 2. Clean up any leftover background proxies, Steam, and hung game/crash handler processes
pkill -f mitmdump || true
pkill -f mitmproxy || true
echo "🧹 Cleanly terminating any running/hung Steam, VRChat, or Crash Handler processes..."
pkill -9 steam || true
pkill -f VRChat || true
pkill -f UnityCrashHandler64 || true
pkill -f start_protected_game || true
sleep 1.0

# 3. Dynamically locate VRChat's Proton Wine prefix path and cleanse any legacy persistent global Wine proxy settings
# This ensures that standard HTTP/HTTPS CDN and image loaders bypass the proxy and run at native gigabit speed!
WINEPREFIX_CANDIDATES=(
    "$HOME/.steam/steam/steamapps/compatdata/438100/pfx"
    "$HOME/.local/share/Steam/steamapps/compatdata/438100/pfx"
    "$HOME/.var/app/com.valvesoftware.Steam/.local/share/Steam/steamapps/compatdata/438100/pfx"
)
WINEPREFIX_PATH=""
for p in "${WINEPREFIX_CANDIDATES[@]}"; do
    if [ -d "$p" ]; then
        WINEPREFIX_PATH="$p"
        break
    fi
done

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
    # Print current state for debugging verification
    with open(reg_path, 'r', encoding='utf-8', errors='ignore') as f:
        verified = f.read()
    match = re.search(r'\[Software\\\\Microsoft\\\\Windows\\\\CurrentVersion\\\\Internet Settings\].*?(?=\n\[|$)', verified, re.DOTALL)
    if match:
        print('🔍 Current Wine Internet Settings Registry State:')
        print(match.group(0).strip())
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

        # Consolidate and merge any downloaded files from legacy folders
        for suffix in ['game-op-original', 'game-op-optimized', 'game-op-assets-optimized', 'game-op-mesh-optimized']:
            legacy_path = cache_path + '.' + suffix
            if os.path.exists(legacy_path) and os.path.isdir(legacy_path):
                print(f'  📦 Consolidating files from legacy folder: {legacy_path}...')
                for root, dirs, files in os.walk(legacy_path):
                    for file in files:
                        src = os.path.join(root, file)
                        rel = os.path.relpath(src, legacy_path)
                        dest = os.path.join(cache_path, rel)
                        os.makedirs(os.path.dirname(dest), exist_ok=True)
                        if not os.path.exists(dest):
                            shutil.move(src, dest)
                shutil.rmtree(legacy_path)

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
    print('✅ AppData LocalLow folders self-healed and permissions restored successfully!')
"
fi

# 4. Automatically copy the mitmproxy certificate to a non-hidden path
# This bypasses the Proton container sandbox/pressure-vessel block on hidden dotfiles
echo "🔒 Preparing certificate for the Proton sandbox..."
mkdir -p "$HOME/.mitmproxy"
if [ ! -f "$HOME/.mitmproxy/mitmproxy-ca-cert.pem" ]; then
    echo "🔑 Generating local security certificates..."
    mitmdump --version > /dev/null 2>&1 || true
    # Run a quick mitmdump instance to ensure cert files are fully written
    mitmdump & sleep 1; kill $! > /dev/null 2>&1 || true
fi
if [ -f "$HOME/.mitmproxy/mitmproxy-ca-cert.pem" ]; then
    cp "$HOME/.mitmproxy/mitmproxy-ca-cert.pem" "$DIR/mitmproxy-ca-cert.pem"
    echo "✅ Certificate copied to non-hidden path: $DIR/mitmproxy-ca-cert.pem"
else
    echo "⚠️ Warning: mitmproxy certificate not found at $HOME/.mitmproxy/mitmproxy-ca-cert.pem"
fi

# Ensure any legacy cache redirection symlinks are reverted to a normal folder to prevent game crashes
if [ -f "./target/release/game-op" ]; then
    echo "🧹 Restoring cache folder to native state to prevent Unity crashes..."
    ./target/release/game-op --revert-cache > /dev/null 2>&1 || true
fi

# 5. Start the headless proxy sniffer in the background with selective host bypass
# We decrypt only api.vrchat.cloud to capture keys, while bypassing files.vrchat.cloud 
# (which has strict TLS pinning and must pass through natively to prevent handshake alerts)
echo "🔒 Starting secure key sniffer proxy (silent mode on port $PROXY_PORT)..."
mitmdump -s asset_key_resolver.py --listen-port $PROXY_PORT --allow-hosts "api\.vrchat\.cloud" > sniffer.log 2>&1 &
PROXY_PID=$!

# Start a background live bundle optimizer loop
# It utilizes Linux kernel 'inotify' interrupts to detect completed downloads instantly,
# optimizing assets in-place in microseconds before VRChat's renderer loads them!
CACHE_PATH="$HOME/.steam/steam/steamapps/compatdata/438100/pfx/drive_c/users/steamuser/AppData/LocalLow/VRChat/VRChat/Cache-WindowsPlayer"

if command -v inotifywait &> /dev/null; then
    echo "⚡ [Kernel Link] Linux inotifywait detected! Enabling instant real-time asset optimization..."
    (
        # Monitor the cache folder recursively for closed-after-write events on __data files
        inotifywait -m -r -e close_write --format "%w%f" "$CACHE_PATH" 2>/dev/null | while read -r filepath; do
            if [[ "$filepath" == *"__data" ]]; then
                # Instantly transcode the completed bundle in-place before the game loads it!
                python3 bundle_optimizer.py "$filepath" "$filepath" 1024 >/dev/null 2>&1 || true
            fi
        done
    ) &
    LIVE_OPT_PID=$!
else
    echo "💡 Notice: Install 'inotify-tools' (sudo apt install inotify-tools) to enable instant kernel-level real-time asset optimization!"
    echo "🔄 Falling back to high-performance 15-second periodic live sweeper..."
    (
        while kill -0 $PROXY_PID 2>/dev/null; do
            sleep 15
            if [ -d "$CACHE_PATH" ]; then
                python3 bundle_optimizer.py "$CACHE_PATH" "$CACHE_PATH" 1024 >/dev/null 2>&1 || true
            fi
        done
    ) &
    LIVE_OPT_PID=$!
fi

# Ensure the proxy turns off cleanly when this script exits/interrupts
cleanup() {
    echo ""
    echo "=================================================================="
    echo " 🧹 Shutting down session..."
    echo "=================================================================="
    if [ -n "$LIVE_OPT_PID" ]; then
        echo "Stopping background live bundle optimizer (PID $LIVE_OPT_PID)..."
        kill "$LIVE_OPT_PID" || true
    fi
    if [ -n "$PROXY_PID" ]; then
        echo "Stopping secure sniffer proxy (PID $PROXY_PID)..."
        kill "$PROXY_PID" || true
    fi
}
trap cleanup EXIT

sleep 1.0

# 6. Debugging Print of any existing host system proxy environment variables
echo "🔍 Checking active terminal environment proxy variables..."
echo "  http_proxy:  $http_proxy"
echo "  https_proxy: $https_proxy"
echo "  all_proxy:   $all_proxy"
echo "  no_proxy:    $no_proxy"
echo ""

# 7. Launch Steam with complete proxy and SSL certificate integration (avoids freezing)
echo "🎮 Launching Steam..."
echo "👉 NOTE: Please ensure your VRChat Steam Launch Options are set to exactly:"
echo "   SSL_CERT_FILE=/home/koronet/Game-Op/mitmproxy-ca-cert.pem http_proxy=http://127.0.0.1:8080 https_proxy=http://127.0.0.1:8080 no_proxy=files.vrchat.cloud,assets.vrchat.cloud,images.vrchat.cloud,pipeline.vrchat.cloud %command%"
echo ""

steam steam://rungameid/438100 > /dev/null 2>&1 &

# 8. Launch the Rust thread booster (blocks until VRChat exits)
echo "⚡ Starting Game-Op OS booster & process priority tracker..."
./target/release/game-op --profile vrchat-hq-low-end --verbose --once

echo "✅ VRChat has closed!"

# 9. Run the in-place asset bundle optimizer (now runs in parallel ProcessPool!)
echo ""
echo "=================================================================="
echo " ⚙️ Deep Bundle Optimization Sweep..."
echo "=================================================================="
CACHE_PATH="$HOME/.steam/steam/steamapps/compatdata/438100/pfx/drive_c/users/steamuser/AppData/LocalLow/VRChat/VRChat/Cache-WindowsPlayer"

python3 bundle_optimizer.py "$CACHE_PATH" "$CACHE_PATH" 1024

echo "=================================================================="
echo " 🎉 Optimization Sweep Completed! Ready for your next session."
echo "=================================================================="
