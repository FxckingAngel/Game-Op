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
