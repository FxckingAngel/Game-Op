#!/bin/bash
# ==================================================================
# Game-Op Black-Box Binary Compiler (Base64 Direct-Exec Packaging)
# ==================================================================
set -e

DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
cd "$DIR"

echo "=================================================================="
echo " ⚙️ Compiling Game-Op Black-Box Base64 Direct-Exec Loaders..."
echo "=================================================================="

# 1. Clean previous build files
rm -rf build/ *.c *.so *.pyc __pycache__ _setup_*_temp.py || true

# 2. Package asset_key_resolver.py
if [ -f "asset_key_resolver_bin.py" ]; then
    python3 -c "
import base64
with open('asset_key_resolver_bin.py', 'r') as f:
    code = f.read()
encoded = base64.b64encode(code.encode('utf-8')).decode('utf-8')
loader = f'''#!/usr/bin/env python3
import sys
import os
import base64

DIR = os.path.dirname(os.path.abspath(__file__))
sys.path.append(DIR)

_c = \"\"\"{encoded}\"\"\"

_g = {{}}
exec(base64.b64decode(_c).decode('utf-8'), _g)
request = _g['request']
response = _g['response']
'''
with open('asset_key_resolver.py', 'w') as f:
    f.write(loader)
print('✅ Packaged asset_key_resolver.py successfully!')
"
fi

# 3. Package bundle_optimizer.py
if [ -f "bundle_optimizer_bin.py" ]; then
    python3 -c "
import base64
with open('bundle_optimizer_bin.py', 'r') as f:
    code = f.read()
encoded = base64.b64encode(code.encode('utf-8')).decode('utf-8')
loader = f'''#!/usr/bin/env python3
import sys
import os
import base64

DIR = os.path.dirname(os.path.abspath(__file__))
sys.path.append(DIR)

_c = \"\"\"{encoded}\"\"\"

_g = {{}}
exec(base64.b64decode(_c).decode('utf-8'), _g)
load_encrypted_db = _g['load_encrypted_db']
if __name__ == '__main__':
    _g['main']()
'''
with open('bundle_optimizer.py', 'w') as f:
    f.write(loader)
print('✅ Packaged bundle_optimizer.py successfully!')
"
fi

echo "=================================================================="
echo " 🎉 Base64 Packaging Complete! Direct-exec loaders written."
echo "=================================================================="
