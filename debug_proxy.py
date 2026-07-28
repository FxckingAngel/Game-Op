#!/usr/bin/env python3
import urllib.request
import urllib.error
import ssl
import os
import sys

def debug():
    print("==================================================")
    print(" 🛠️ Game-Op proxy diagnostic system")
    print("==================================================")
    
    db_path = os.path.expanduser("~/Game-Op/vrc_keys.db")
    log_path = os.path.expanduser("~/Game-Op/sniffer.log")
    
    # 1. Check local key database state
    print(f"Checking file database paths:")
    for path, name in [(db_path, "vrc_keys.db"), (log_path, "sniffer.log")]:
        if os.path.exists(path):
            print(f"  ✅ {name} exists, size: {os.path.getsize(path)} bytes")
        else:
            print(f"  ❌ {name} does not exist!")

    # 2. Check if mitmdump is running
    print(f"\nChecking local proxy state:")
    import socket
    s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    s.settimeout(1.0)
    try:
        s.connect(("127.0.0.1", 8080))
        print("  ✅ Proxy port 8080 is actively listening!")
        s.close()
    except Exception as e:
        print(f"  ❌ Proxy port 8080 is not listening! Error: {e}")
        print("  Please ensure ./start_vrc.sh is running in another window.")
        sys.exit(1)

    # 3. Test secure SSL proxy loopback to api.vrchat.cloud
    print("\nTesting SSL Handshake & Certificate Verification through proxy:")
    proxy_handler = urllib.request.ProxyHandler({
        'http': 'http://127.0.0.1:8080',
        'https': 'http://127.0.0.1:8080'
    })
    opener = urllib.request.build_opener(proxy_handler)
    
    # Set standard User-Agent
    opener.addheaders = [('User-Agent', 'Mozilla/5.0 (Windows NT 10.0; Win64; x64)')]
    
    test_url = "https://api.vrchat.cloud/api/1/config"
    print(f"Connecting to {test_url} through proxy...")
    
    try:
        # Try secure request (should succeed if system cert store trusts mitmproxy CA)
        with opener.open(test_url, timeout=5.0) as response:
            html = response.read()
            print("  🎉 SUCCESS! SSL connection is trusted and verified perfectly through the proxy!")
            print("  This means your system certificate store is 100% configured correctly.")
    except urllib.error.HTTPError as e:
        # Since we got an HTTP status code back from the secure server, the SSL Handshake succeeded!
        print("  🎉 SUCCESS! SSL connection is trusted and verified perfectly through the proxy!")
        print(f"  (Note: Server returned HTTP {e.code} Forbidden, which is expected for anonymous API requests without VRChat client headers).")
    except urllib.error.URLError as e:
        print(f"  ❌ CONNECTION FAILED: {e}")
        reason = e.reason
        if isinstance(reason, ssl.SSLError):
            print("\n🚨 DIAGNOSIS: SSL Handshake Verification Error!")
            print("  This means VRChat/Steam does not trust your mitmproxy Root CA certificate.")
            print("  Please run the following commands to install and trust the certificate system-wide:")
            print("\n  👉 sudo cp ~/.mitmproxy/mitmproxy-ca-cert.pem /usr/local/share/ca-certificates/mitmproxy.crt")
            print("  👉 sudo update-ca-certificates")
        else:
            print(f"\n🚨 DIAGNOSIS: Network connection error: {reason}")
    except Exception as e:
        print(f"  ❌ UNKNOWN ERROR: {e}")

    # 4. Check sniffer.log for internal errors
    if os.path.exists(log_path):
        print("\nChecking last 10 lines of sniffer.log for errors:")
        with open(log_path, "r") as f:
            lines = f.readlines()
            for line in lines[-10:]:
                print(f"  [Log] {line.strip()}")

    print("==================================================")

if __name__ == "__main__":
    debug()
