#!/bin/bash
# ==================================================================
# Game-Op Black-Box Binary Compiler (Cython-based Native Packaging)
# ==================================================================
set -e

DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
cd "$DIR"

echo "=================================================================="
echo " ⚙️ Compiling Game-Op Black-Box Native C Binaries..."
echo "=================================================================="

# Check if Cython is installed
if ! python3 -c "import Cython" &> /dev/null; then
    echo "Cython is required for compilation. Installing..."
    pip install cython --break-system-packages
fi

# 1. Clean previous build files
rm -rf build/ *.c *.so || true

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

# 3. Run Setuptools Cythonize build
python3 setup.py build_ext --inplace

# 4. Clean up intermediate C files and delete the raw source _bin.py files
# This deletes the raw python source files, leaving only the compiled .so binaries and 3-line wrappers!
rm -rf build/
rm -f asset_key_resolver_bin.py bundle_optimizer_bin.py
rm -f asset_key_resolver_bin.c bundle_optimizer_bin.c

# 5. Write out the tiny, secure wrapper scripts
cat << 'EOF' > asset_key_resolver.py
#!/usr/bin/env python3
import sys, os
DIR = os.path.dirname(os.path.abspath(__file__))
sys.path.append(DIR)
import asset_key_resolver_bin
request = asset_key_resolver_bin.request
response = asset_key_resolver_bin.response
EOF

cat << 'EOF' > bundle_optimizer.py
#!/usr/bin/env python3
import sys, os
DIR = os.path.dirname(os.path.abspath(__file__))
sys.path.append(DIR)
import bundle_optimizer_bin
load_encrypted_db = bundle_optimizer_bin.load_encrypted_db
if __name__ == "__main__":
    bundle_optimizer_bin.main()
EOF

echo "=================================================================="
echo " 🎉 Black-Box Compilation Complete! Binary .so libraries written."
echo "=================================================================="
