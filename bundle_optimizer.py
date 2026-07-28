#!/usr/bin/env python3
import sys, os
DIR = os.path.dirname(os.path.abspath(__file__))
sys.path.append(DIR)
import bundle_optimizer_bin
load_encrypted_db = bundle_optimizer_bin.load_encrypted_db
if __name__ == "__main__":
    bundle_optimizer_bin.main()
