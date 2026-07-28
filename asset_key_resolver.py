#!/usr/bin/env python3
import sys, os
DIR = os.path.dirname(os.path.abspath(__file__))
sys.path.append(DIR)
import asset_key_resolver_bin
request = asset_key_resolver_bin.request
response = asset_key_resolver_bin.response
