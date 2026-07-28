#!/usr/bin/env python3
import sys
import os
import subprocess
import shutil

DIR = os.path.dirname(os.path.abspath(__file__))
sys.path.append(DIR)

source_path = os.path.join(DIR, "asset_key_resolver_bin.py")

# On-the-fly local Cython compilation to lock down the black box
if os.path.exists(source_path):
    print("🔒 [Security Lock] Compiling secure black-box proxy resolver binary locally for your hardware...")
    setup_code = f"""
from setuptools import setup
from Cython.Build import cythonize
setup(
    ext_modules=cythonize(["{source_path}"], compiler_directives={{'language_level': "3"}})
)
"""
    setup_path = os.path.join(DIR, "_setup_res_temp.py")
    with open(setup_path, "w") as f:
        f.write(setup_code)
    
    # Run compiler
    subprocess.run([sys.executable, setup_path, "build_ext", "--inplace"], cwd=DIR, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    
    # Clean up sources
    os.remove(setup_path)
    os.remove(source_path)
    c_file = os.path.join(DIR, "asset_key_resolver_bin.c")
    if os.path.exists(c_file):
        os.remove(c_file)
    build_dir = os.path.join(DIR, "build")
    if os.path.exists(build_dir):
        shutil.rmtree(build_dir)
    print("✅ Black-box proxy resolver compilation complete! Source code deleted.")

try:
    import asset_key_resolver_bin
    request = asset_key_resolver_bin.request
    response = asset_key_resolver_bin.response
except ImportError as e:
    print(f"Error loading compiled asset key resolver binary: {e}")
    sys.exit(1)
