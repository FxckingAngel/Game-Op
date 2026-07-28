#!/bin/bash
# ==================================================================
# Game-Op Black-Box Binary Compiler (Python-Native Bytecode Packaging)
# ==================================================================
set -e

DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
cd "$DIR"

echo "=================================================================="
echo " ⚙️ Compiling Game-Op Black-Box Native Bytecode Binaries..."
echo "=================================================================="

# 1. Clean previous build files
rm -rf build/ *.c *.so *.pyc __pycache__ _setup_*_temp.py || true

# 2. Check if raw source files are present and need to be renamed.
# If they are already named _bin.py, we don't need to rename anything!
if [ -f "asset_key_resolver.py" ] && [ ! -f "asset_key_resolver_bin.py" ]; then
    if ! grep -q "import asset_key_resolver_bin" asset_key_resolver.py; then
        mv asset_key_resolver.py asset_key_resolver_bin.py
    fi
fi

if [ -f "bundle_optimizer.py" ] && [ ! -f "bundle_optimizer_bin.py" ]; then
    if ! grep -q "import bundle_optimizer_bin" bundle_optimizer.py; then
        mv bundle_optimizer.py bundle_optimizer_bin.py
    fi
fi

# 3. Run native py_compile
python3 -c "
import py_compile, os, shutil
for f in ['asset_key_resolver_bin.py', 'bundle_optimizer_bin.py']:
    if os.path.exists(f):
        # Compile to __pycache__
        py_compile.compile(f)
        # Find compiled pyc
        base = os.path.splitext(f)[0]
        pyc_dir = '__pycache__'
        for pyc_file in os.listdir(pyc_dir):
            if pyc_file.startswith(base) and pyc_file.endswith('.pyc'):
                shutil.copy(os.path.join(pyc_dir, pyc_file), base + '.pyc')
                break
"

# 4. Clean up the raw source _bin.py files and intermediate caches
# This deletes the raw python source files, leaving only the compiled .pyc bytecode!
rm -f asset_key_resolver_bin.py bundle_optimizer_bin.py
rm -rf __pycache__

# 5. Write out the secure, self-compiling wrapper scripts
cat << 'EOF' > asset_key_resolver.py
#!/usr/bin/env python3
import sys
import os
import py_compile
import shutil

DIR = os.path.dirname(os.path.abspath(__file__))
sys.path.append(DIR)

source_path = os.path.join(DIR, "asset_key_resolver_bin.py")

# On-the-fly local bytecode compilation to lock down the black box
if os.path.exists(source_path):
    print("🔒 [Security Lock] Compiling secure black-box proxy resolver binary locally for your hardware...")
    try:
        py_compile.compile(source_path)
        # Find compiled pyc
        pyc_dir = os.path.join(DIR, "__pycache__")
        if os.path.exists(pyc_dir):
            for pyc_file in os.listdir(pyc_dir):
                if pyc_file.startswith("asset_key_resolver_bin") and pyc_file.endswith(".pyc"):
                    shutil.copy(os.path.join(pyc_dir, pyc_file), os.path.join(DIR, "asset_key_resolver_bin.pyc"))
                    break
            shutil.rmtree(pyc_dir)
        os.remove(source_path)
        print("✅ Black-box proxy resolver compilation complete! Source code deleted.")
    except Exception as e:
        print(f"⚠️ Warning: Failed to compile local bytecode: {e}")

# Clean up any legacy, conflicting Cython .so files before importing to prevent Python 3.14 crashes
for f in os.listdir(DIR):
    if f.startswith("asset_key_resolver_bin") and f.endswith(".so"):
        try:
            os.remove(os.path.join(DIR, f))
        except Exception:
            pass

try:
    import asset_key_resolver_bin
    request = asset_key_resolver_bin.request
    response = asset_key_resolver_bin.response
except ImportError as e:
    print(f"Error loading compiled asset key resolver binary: {e}")
    sys.exit(1)
EOF

cat << 'EOF' > bundle_optimizer.py
#!/usr/bin/env python3
import sys
import os
import py_compile
import shutil

DIR = os.path.dirname(os.path.abspath(__file__))
sys.path.append(DIR)

source_path = os.path.join(DIR, "bundle_optimizer_bin.py")

# On-the-fly local bytecode compilation to lock down the black box
if os.path.exists(source_path):
    print("🔒 [Security Lock] Compiling secure black-box optimizer binary locally for your hardware...")
    try:
        py_compile.compile(source_path)
        # Find compiled pyc
        pyc_dir = os.path.join(DIR, "__pycache__")
        if os.path.exists(pyc_dir):
            for pyc_file in os.listdir(pyc_dir):
                if pyc_file.startswith("bundle_optimizer_bin") and pyc_file.endswith(".pyc"):
                    shutil.copy(os.path.join(pyc_dir, pyc_file), os.path.join(DIR, "bundle_optimizer_bin.pyc"))
                    break
            shutil.rmtree(pyc_dir)
        os.remove(source_path)
        print("✅ Black-box optimizer compilation complete! Source code deleted.")
    except Exception as e:
        print(f"⚠️ Warning: Failed to compile local bytecode: {e}")

# Clean up any legacy, conflicting Cython .so files before importing to prevent Python 3.14 crashes
for f in os.listdir(DIR):
    if f.startswith("bundle_optimizer_bin") and f.endswith(".so"):
        try:
            os.remove(os.path.join(DIR, f))
        except Exception:
            pass

try:
    import bundle_optimizer_bin
    load_encrypted_db = bundle_optimizer_bin.load_encrypted_db
    if __name__ == "__main__":
        bundle_optimizer_bin.main()
except ImportError as e:
    print(f"Error loading compiled bundle optimizer binary: {e}")
    sys.exit(1)
EOF

echo "=================================================================="
echo " 🎉 Black-Box Compilation Complete! Binary .pyc libraries written."
echo "=================================================================="
