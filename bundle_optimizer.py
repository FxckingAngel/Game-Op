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

try:
    import bundle_optimizer_bin
    load_encrypted_db = bundle_optimizer_bin.load_encrypted_db
    if __name__ == "__main__":
        bundle_optimizer_bin.main()
except ImportError as e:
    print(f"Error loading compiled bundle optimizer binary: {e}")
    sys.exit(1)
