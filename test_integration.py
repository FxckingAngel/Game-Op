#!/usr/bin/env python3
import subprocess
import time
import os
import sys
import urllib.request
import urllib.error
import ssl

# Add the current directory to sys.path to import bundle_optimizer
sys.path.append(os.path.dirname(os.path.abspath(__file__)))
from bundle_optimizer import load_encrypted_db

def run_integration_test():
    print("==================================================")
    print(" 🧪 Starting Game-Op End-to-End Integration Test")
    print("==================================================")
    
    # 1. Clean up any existing test files
    db_path = os.path.expanduser("~/Game-Op/vrc_keys.db")
    
    if os.path.exists(db_path):
        os.remove(db_path)
        
    # 2. Start mitmdump with asset_key_resolver.py on port 8080
    print("Spawning mitmdump secure key sniffer proxy on port 8080...")
    proxy_proc = subprocess.Popen([
        "mitmdump",
        "-s", "asset_key_resolver.py",
        "--listen-port", "8080",
        "--allow-hosts", "api\\.vrchat\\.cloud"
    ], stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    
    # Wait for the proxy to start up
    time.sleep(2.0)
    
    # Verify the proxy is running
    if proxy_proc.poll() is not None:
        print("❌ Error: mitmdump failed to start!")
        stdout, stderr = proxy_proc.communicate()
        print(f"Stdout: {stdout.decode()}")
        print(f"Stderr: {stderr.decode()}")
        sys.exit(1)
        
    print("✅ mitmdump proxy spawned successfully.")
    
    # 3. Perform HTTPS request through the proxy to api.vrchat.cloud
    # Since our key_sniffer.py request() hook intercepts "file_12345678-abcd-1234-abcd-1234567890ab"
    # and returns a mock JSON response, this will succeed offline without hitting the real internet!
    proxy_handler = urllib.request.ProxyHandler({
        'http': 'http://127.0.0.1:8080',
        'https': 'http://127.0.0.1:8080'
    })
    
    # Disable SSL context verification for python client because we are self-signing / using mitmproxy's local CA
    # which is trusted by the host, but python's default ssl context might need to be explicitly configured or bypassed.
    ssl_context = ssl.create_default_context()
    ssl_context.check_hostname = False
    ssl_context.verify_mode = ssl.CERT_NONE
    
    opener = urllib.request.build_opener(proxy_handler, urllib.request.HTTPSHandler(context=ssl_context))
    opener.addheaders = [('User-Agent', 'Mozilla/5.0 (Windows NT 10.0; Win64; x64)')]
    
    test_url = "https://api.vrchat.cloud/api/1/avatars/file_12345678-abcd-1234-abcd-1234567890ab"
    print(f"Sending test HTTPS request through the proxy to: {test_url}")
    
    try:
        with opener.open(test_url, timeout=5.0) as response:
            res_data = response.read().decode("utf-8")
            print(f"✅ Received Response: {res_data}")
    except Exception as e:
        print(f"❌ Connection failed: {e}")
        proxy_proc.terminate()
        stdout, stderr = proxy_proc.communicate()
        print(f"--- Proxy stdout ---\n{stdout.decode()}")
        print(f"--- Proxy stderr ---\n{stderr.decode()}")
        sys.exit(1)
        
    # Wait for the sniffer to process the response and write to the database
    time.sleep(2.0)
    
    # 4. Terminate the mitmdump proxy
    print("Terminating mitmdump proxy...")
    proxy_proc.terminate()
    proxy_proc.wait()
    
    # 5. Verify database exists
    print("Verifying cryptographic files:")
    if os.path.exists(db_path):
        print(f"  ✅ vrc_keys.db exists, size: {os.path.getsize(db_path)} bytes")
    else:
        print("  ❌ vrc_keys.db does not exist!")
        sys.exit(1)
        
    # 6. Load and decrypt the database
    print("Decrypting secure key database...")
    keys_db = load_encrypted_db(db_path, None)
    
    print(f"Decrypted database contents: {keys_db}")
    
    # Clean target ID
    target_clean_id = "12345678ABCD1234ABCD1234567890AB"
    expected_key = "0123456789abcdef0123456789abcdef"
    
    if target_clean_id in keys_db:
        decrypted_key = keys_db[target_clean_id]
        print(f"  ✅ Decrypted key found: {decrypted_key}")
        if decrypted_key == expected_key:
            print("🎉 INTEGRATION TEST PASSED! Security & Key Decryption verified successfully!")
        else:
            print(f"❌ Error: Decrypted key '{decrypted_key}' does not match expected key '{expected_key}'!")
            sys.exit(1)
    else:
        print(f"❌ Error: Target file ID {target_clean_id} was not found in decrypted database!")
        sys.exit(1)

if __name__ == "__main__":
    run_integration_test()
