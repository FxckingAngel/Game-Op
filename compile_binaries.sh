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
echo " 🎉 Black-Box Compilation Complete! Binary .pyc libraries written."
echo "=================================================================="
