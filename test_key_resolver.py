#!/usr/bin/env python3
# ==================================================================
# Game-Op VRChat Asset Decryption Key Resolver & Diagnostics Tool
# ==================================================================
import os
import sys
import re
import shutil
import sqlite3
import json
import urllib.request
import ssl

def load_vrc_auth_cookie():
    """
    Scans common Steam/Proton paths to find the active VRChat Cookies SQLite file,
    and extracts the authenticated login session cookie using SQLite or raw string fallback.
    """
    candidates = [
        os.path.expanduser("~/.steam/steam/steamapps/compatdata/438100/pfx/drive_c/users/steamuser/AppData/LocalLow/VRChat/VRChat"),
        os.path.expanduser("~/.local/share/Steam/steamapps/compatdata/438100/pfx/drive_c/users/steamuser/AppData/LocalLow/VRChat/VRChat"),
        os.path.expanduser("~/.var/app/com.valvesoftware.Steam/.local/share/Steam/steamapps/compatdata/438100/pfx/drive_c/users/steamuser/AppData/LocalLow/VRChat/VRChat"),
    ]
    
    cookies_paths = []
    for base in candidates:
        if os.path.exists(base):
            cookies_paths.append(os.path.join(base, "Cookies"))
            cookies_paths.append(os.path.join(base, "Cookies", "Cookies"))
            for root, dirs, files in os.walk(base):
                for f in files:
                    if "cookie" in f.lower():
                        cookies_paths.append(os.path.join(root, f))
                        
    # Filter only existing files
    cookies_paths = [p for p in cookies_paths if os.path.exists(p) and os.path.isfile(p)]
    
    if not cookies_paths:
        print("⚠️  [Cookie Resolver] VRChat Cookies file not found on standard paths.")
        return None
        
    for path in cookies_paths:
        # Method 1: Copy to /tmp to bypass locking and query SQLite
        temp_path = "/tmp/vrc_cookies_resolve_temp"
        try:
            shutil.copy(path, temp_path)
            conn = sqlite3.connect(temp_path)
            cursor = conn.cursor()
            cursor.execute("SELECT name, value, host_key FROM cookies WHERE name LIKE '%auth%'")
            for name, value, host in cursor.fetchall():
                if "auth" in name.lower() and "vrchat" in host.lower():
                    conn.close()
                    try: os.remove(temp_path)
                    except Exception: pass
                    return f"{name}={value}"
            conn.close()
            try: os.remove(temp_path)
            except Exception: pass
        except Exception:
            try: os.remove(temp_path)
            except Exception: pass
            
        # Method 2: Fallback to raw plaintext regex (extremely robust if SQLite is locked/corrupted)
        try:
            with open(path, "r", encoding="utf-8", errors="ignore") as f:
                content = f.read()
                match = re.search(r"(authcookie_[0-9a-fA-F\-]+=[0-9a-fA-F\-]+)", content)
                if match:
                    return match.group(1)
        except Exception:
            pass
            
    return None

def fetch_avatar_metadata(avatar_id, cookie):
    """
    Queries VRChat's API to fetch the avatar's metadata including its file_ID.
    """
    url = f"https://api.vrchat.cloud/api/1/avatars/{avatar_id}"
    api_key = "JlE5Jldo5Jibnk5O5hTx6XVqsJu4WJ26"
    req_url = f"{url}?apiKey={api_key}"
    
    req = urllib.request.Request(req_url)
    req.add_header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
    req.add_header("Origin", "https://vrchat.com")
    req.add_header("Referer", "https://vrchat.com/")
    if cookie:
        req.add_header("Cookie", cookie)
        
    ssl_context = ssl.create_default_context()
    ssl_context.check_hostname = False
    ssl_context.verify_mode = ssl.CERT_NONE
    
    with urllib.request.urlopen(req, context=ssl_context, timeout=5.0) as r:
        return json.loads(r.read().decode("utf-8"))

def fetch_file_key(file_id, cookie):
    """
    Queries VRChat's API to fetch the asset's decryption key using the file_ID.
    """
    url = f"https://api.vrchat.cloud/api/1/file/{file_id}"
    api_key = "JlE5Jldo5Jibnk5O5hTx6XVqsJu4WJ26"
    req_url = f"{url}?apiKey={api_key}"
    
    req = urllib.request.Request(req_url)
    req.add_header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
    req.add_header("Origin", "https://vrchat.com")
    req.add_header("Referer", "https://vrchat.com/")
    if cookie:
        req.add_header("Cookie", cookie)
        
    ssl_context = ssl.create_default_context()
    ssl_context.check_hostname = False
    ssl_context.verify_mode = ssl.CERT_NONE
    
    with urllib.request.urlopen(req, context=ssl_context, timeout=5.0) as r:
        data = json.loads(r.read().decode("utf-8"))
        versions = data.get("versions", [])
        for version in reversed(versions): # Prefer latest version
            for key_name in ["file", "delta"]:
                pkg = version.get(key_name, {})
                key = pkg.get("decryptionKey") or pkg.get("unityKey") or pkg.get("assetKey")
                if key and len(key) >= 32:
                    return key, version.get("version", 1)
    return None, None

def main():
    avatar_id = sys.argv[1] if len(sys.argv) > 1 else "avtr_70d050e8-1a27-4a77-b7f0-aef8919d64bb"
    
    print("==================================================================")
    print(" 🛰️  GAME-OP LIVE KEY RESOLVER DIAGNOSTIC SYSTEM")
    print("==================================================================")
    print(f"Target Avatar ID: {avatar_id}")
    
    # 1. Load active login cookie
    print("\n[1/3] Loading active VRChat auth session cookie...")
    cookie = load_vrc_auth_cookie()
    if cookie:
        # Obfuscate cookie values to print safely in logs
        masked_cookie = re.sub(r"=(authcookie_[0-9a-fA-F\-]{6})[a-zA-Z0-9\-]+", r"=\1****************", cookie)
        print(f"  ✅ Cookie Loaded Successfully: {masked_cookie}")
    else:
        print("  ❌ ERROR: No active VRChat login cookie found! Please log into VRChat first.")
        sys.exit(1)
        
    # 2. Fetch Avatar Metadata to get File ID
    print("\n[2/3] Querying VRChat API for avatar metadata...")
    try:
        meta = fetch_avatar_metadata(avatar_id, cookie)
        name = meta.get("name", "Unknown Name")
        author = meta.get("authorName", "Unknown Author")
        asset_url = meta.get("assetUrl", "") or meta.get("unityPackageUrl", "")
        
        print(f"  ✅ Avatar Found: '{name}' by {author}")
        
        # Extract file_ID from assetUrl
        file_id_match = re.search(r"(file_[0-9a-fA-F\-]+)", asset_url)
        if file_id_match:
            file_id = file_id_match.group(1)
            print(f"  ✅ Extracted File ID: {file_id}")
        else:
            print("  ❌ ERROR: Could not extract file ID from assetUrl! AssetUrl may be empty or restricted.")
            sys.exit(1)
    except Exception as e:
        print(f"  ❌ ERROR: Failed to retrieve avatar metadata: {e}")
        sys.exit(1)
        
    # 3. Fetch Decryption Key using File ID
    print("\n[3/3] Querying VRChat API for asset decryption key...")
    try:
        key, version = fetch_file_key(file_id, cookie)
        if key:
            print(f"  🎉 SUCCESS! Decryption key retrieved successfully!")
            
            print("\n" + "="*66)
            print(" 🔑 VRC_KEYS CRYPTOGRAPHIC DECRYPTION KEY REPORT")
            print("="*66)
            print(f"| Avatar Name:      {name:<46} |")
            print(f"| Creator/Author:   {author:<46} |")
            print(f"| File ID:          {file_id:<46} |")
            print(f"| Asset Version:    v{version:<45} |")
            print(f"| DECRYPTION KEY:   {key:<46} |")
            print("="*66)
            print("\n✅ Key has been successfully retrieved!")
            
            # Save resolved key to user's database directly!
            db_path = os.path.expanduser("~/Game-Op/vrc_keys.db")
            if os.path.exists(os.path.dirname(db_path)):
                from bundle_optimizer import append_key_to_db
                append_key_to_db(file_id, key, db_path)
                
        else:
            print("  ❌ ERROR: No decryption key returned by API! The asset may not be encrypted, or is restricted.")
    except Exception as e:
        print(f"  ❌ ERROR: Failed to retrieve asset decryption key: {e}")
        sys.exit(1)

if __name__ == "__main__":
    main()
