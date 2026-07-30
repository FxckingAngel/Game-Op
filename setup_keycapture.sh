#!/bin/bash
# ==================================================================
# Game-Op Key-Capture Setup (run this ONCE)
# ==================================================================
# Installs mitmproxy if needed, generates the local CA certificate, copies it
# next to this script, and prints the exact Steam Launch Options to set. After
# this, ./start_vrc.sh will capture VRChat decryption keys automatically so the
# optimizer can decrypt and shrink your cached bundles.
# ==================================================================
set -e

DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
cd "$DIR"
export PATH="$HOME/.local/bin:$PATH"

echo "=================================================================="
echo " Game-Op Key-Capture Setup"
echo "=================================================================="

# 1. Ensure mitmproxy is available.
if ! command -v mitmdump &> /dev/null; then
    echo "mitmproxy not found. Installing (user site)..."
    python3 -m pip install --user mitmproxy || \
        python3 -m pip install --user --break-system-packages mitmproxy || {
            echo "ERROR: could not install mitmproxy. Install it manually:"
            echo "  python3 -m pip install --user mitmproxy"
            exit 1
        }
fi
if ! command -v mitmdump &> /dev/null; then
    echo "ERROR: mitmdump still not on PATH. Add ~/.local/bin to your PATH and re-run."
    exit 1
fi
echo "mitmproxy: $(mitmdump --version 2>/dev/null | head -1)"

# 2. Generate the CA certificate if it does not exist yet.
SRC_CERT="$HOME/.mitmproxy/mitmproxy-ca-cert.pem"
if [ ! -f "$SRC_CERT" ]; then
    echo "Generating mitmproxy CA certificate..."
    mitmdump --listen-host 127.0.0.1 --listen-port 8099 > /dev/null 2>&1 &
    GEN_PID=$!
    for _ in $(seq 1 20); do
        [ -f "$SRC_CERT" ] && break
        sleep 0.5
    done
    kill "$GEN_PID" > /dev/null 2>&1 || true
fi
if [ ! -f "$SRC_CERT" ]; then
    echo "ERROR: certificate was not generated at $SRC_CERT"
    exit 1
fi

# 3. Copy the cert next to this script (stable path the launcher references).
cp "$SRC_CERT" "$DIR/mitmproxy-ca-cert.pem"
echo "Certificate ready: $DIR/mitmproxy-ca-cert.pem"

# 4. Print the exact Steam Launch Options.
OPTS="SSL_CERT_FILE=\"$DIR/mitmproxy-ca-cert.pem\" MONO_TLS_ALLOW_UNTRUSTED=1 http_proxy=http://127.0.0.1:8080 https_proxy=http://127.0.0.1:8080 no_proxy=\"files.vrchat.cloud,assets.vrchat.cloud,images.vrchat.cloud,pipeline.vrchat.cloud\" DXVK_CONFIG_FILE=$DIR/dxvk.conf DXVK_ASYNC=1 DXVK_FRAME_PACE=low-latency mesa_glthread=true MESA_GL_THREAD_CHANNEL=true MESA_NO_ERROR=1 INTEL_PRECISE_TRIG=0"

echo ""
echo "=================================================================="
echo " ONE-TIME STEP: set VRChat's Steam Launch Options to exactly this"
echo " (Steam > right-click VRChat > Properties > Launch Options):"
echo "=================================================================="
echo ""
echo "$OPTS %command%"
echo ""
echo "=================================================================="
echo "Then just run:  ./start_vrc.sh"
echo "As you load avatars/worlds, keys are captured to ~/Game-Op/vrc_keys.db"
echo "and the optimizer decrypts + shrinks those bundles automatically."
echo "=================================================================="
