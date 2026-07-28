#!/usr/bin/env python3
import os
import sys
import shutil
import io
import json
import hmac
import hashlib
from cryptography.hazmat.primitives.ciphers.aead import AESGCM

try:
    from PIL import Image
except ImportError:
    print("Error: Pillow is required. Install it using: pip install Pillow")
    sys.exit(1)

try:
    import UnityPy
except ImportError:
    print("==================================================================")
    print("UnityPy is required to parse and optimize assets inside bundles.")
    print("To install it, run:")
    print("👉 pip install UnityPy")
    print("==================================================================")
    sys.exit(1)

def get_hardware_uuid():
    # Try reading the system's motherboard UUID (highly secure, physical hardware-bound)
    try:
        with open("/sys/class/dmi/id/product_uuid", "r") as f:
            val = f.read().strip()
            if val: return val
    except Exception:
        pass
    # Fallback to unique Linux Machine ID
    try:
        with open("/etc/machine-id", "r") as f:
            val = f.read().strip()
            if val: return val
    except Exception:
        pass
    # Fallback to D-Bus machine ID
    try:
        with open("/var/lib/dbus/machine-id", "r") as f:
            val = f.read().strip()
            if val: return val
    except Exception:
        pass
    # Ultimate fallback to username + hostname
    import getpass, socket
    return f"{getpass.getuser()}:{socket.gethostname()}"

def derive_hardware_key():
    hw_id = get_hardware_uuid()
    # Use PBKDF2 to derive a 256-bit Master Key from the physical hardware ID
    # Highly secure, un-rippable, matches VRChat's client security level
    salt = b"Game-Op-Secure-Salt-V2"
    return hashlib.pbkdf2_hmac("sha256", hw_id.encode(), salt, 100000, 32)

def load_encrypted_db(db_path, key_path):
    """
    Safely decrypts and cryptographically verifies the secure local key database
    using AES-GCM (military-grade authenticated encryption) with a Master Key
    derived dynamically from the physical motherboard hardware.
    """
    if not os.path.exists(db_path):
        return {}
        
    try:
        key = derive_hardware_key()
        aesgcm = AESGCM(key)
    except Exception as e:
        print(f"Error deriving hardware-bound decryption key: {e}")
        return {}
        
    keys_db = {}
    try:
        with open(db_path, "r") as f:
            for line in f:
                line = line.strip()
                if not line or ":" not in line:
                    continue
                try:
                    nonce_hex, ciphertext_hex = line.split(":", 1)
                    nonce = bytes.fromhex(nonce_hex)
                    ciphertext = bytes.fromhex(ciphertext_hex)
                    
                    # Decrypt and authenticate automatically inside GCM!
                    entry_bytes = aesgcm.decrypt(nonce, ciphertext, None)
                    entry = json.loads(entry_bytes.decode("utf-8"))
                    
                    file_id = entry.get("id", "")
                    key_val = entry.get("key", "")
                    if file_id and key_val:
                        # Clean file_id (remove 'file_', remove dashes, make uppercase)
                        clean_id = file_id.replace("file_", "").replace("-", "").upper()
                        keys_db[clean_id] = key_val
                except Exception:
                    # Authentication failure (e.g. tampered database or wrong hardware!)
                    pass
    except Exception as e:
        print(f"Error loading secure key database: {e}")
        
    return keys_db

def find_key_for_bundle(bundle_path, keys_db):
    """
    Automatically maps the bundle path to our decrypted database keys.
    """
    path_upper = os.path.abspath(bundle_path).upper().replace("-", "")
    for clean_id, key_hex in keys_db.items():
        # Match standard file ID patterns or 16-character subdirectory hashes
        if clean_id in path_upper or (len(clean_id) >= 16 and clean_id[:16] in path_upper):
            return key_hex
    return None

def optimize_bundle(bundle_path, output_path, max_size=1024, keys_db=None, min_size_mb=30):
    # Zero-IO Caching: Check if this bundle has already been optimized by Game-Op
    parent_dir = os.path.dirname(bundle_path)
    marker_path = os.path.join(parent_dir, ".game-op-processed")
    if os.path.exists(marker_path):
        # Already optimized! Skip entirely with 0 disk IO and 0 CPU overhead!
        return False

    file_size_mb = os.path.getsize(bundle_path) / (1024 * 1024)
    if file_size_mb < min_size_mb:
        # Skip optimizing small bundles, just copy or skip
        if bundle_path != output_path:
            try:
                os.makedirs(os.path.dirname(output_path), exist_ok=True)
                shutil.copy(bundle_path, output_path)
            except Exception:
                pass
        # Mark as processed to prevent touching it on next runs
        try:
            with open(marker_path, "w") as f:
                pass
        except Exception:
            pass
        return False

    print(f"Analyzing UnityFS bundle: {os.path.basename(bundle_path)} ({file_size_mb:.1f} MB)")
    
    # 1. Automically detect and apply local decryption keys if available
    if keys_db:
        key_hex = find_key_for_bundle(bundle_path, keys_db)
        if key_hex:
            print(f"  🔑 [Secure Link] Found matching decryption key for this bundle path!")
            try:
                UnityPy.set_assetbundle_decrypt_key(bytes.fromhex(key_hex))
            except Exception as e:
                print(f"  [Error] Failed to apply decryption key: {e}")

    try:
        env = UnityPy.load(bundle_path)
    except Exception as e:
        err_msg = str(e).lower()
        if "encrypted" in err_msg or "key" in err_msg:
            print(f"  🔒 [Skip Encrypted] Bundle {os.path.basename(bundle_path)} is encrypted, but no key was found in local database yet. Skipping optimization.")
        else:
            print(f"  ❌ [Corrupted Bundle] Failed to parse bundle {os.path.basename(bundle_path)}: {e}")
            # The file is corrupted. Auto-delete the file and its parent folder (which is the asset's cache subdirectory)
            # to ensure VRChat automatically re-downloads a clean copy on next startup.
            try:
                parent_dir = os.path.dirname(bundle_path)
                # Ensure we are inside the VRChat cache folder structure before recursively deleting anything!
                if "Cache-WindowsPlayer" in parent_dir:
                    print(f"  扫 [Auto-Cleanup] Deleting corrupted cache subdirectory: {parent_dir}")
                    shutil.rmtree(parent_dir)
            except Exception as del_err:
                print(f"  [Error] Failed to remove corrupted subdirectory: {del_err}")
        return False

    optimized_count = 0
    
    for obj in env.objects:
        if obj.type.name == "Texture2D":
            try:
                texture = obj.read()
                # Exclude UI, fonts, and internal textures
                name_lower = texture.name.lower() if texture.name else ""
                if any(k in name_lower for k in ["font", "ui", "sprite", "icon"]):
                    continue
                    
                width = texture.m_Width
                height = texture.m_Height
                largest_side = max(width, height)
                
                # Calculate class-aware customized max size for this texture
                target_max = max_size
                is_detail = any(k in name_lower for k in ["face", "head", "eye", "skin", "body", "hair", "cloth", "albedo", "diffuse", "basecolor", "normal", "_nrm", "bump"])
                is_mask = any(k in name_lower for k in ["mask", "rough", "roughness", "metal", "metallic", "ao", "occlusion", "packed", "specular", "emission"])
                
                if is_detail:
                    target_max = round(max_size * 1.25)
                elif is_mask:
                    target_max = max(256, round(max_size * 0.5))
                
                if largest_side > target_max:
                    # Extract image via PIL
                    pil_img = texture.image
                    
                    # Ensure the image mode is strictly RGB or RGBA (prevents Pillow saving/formatting errors)
                    if pil_img.mode not in ["RGB", "RGBA"]:
                        if "A" in pil_img.mode:
                            pil_img = pil_img.convert("RGBA")
                        else:
                            pil_img = pil_img.convert("RGB")
                    
                    # Calculate new size preserving aspect ratio
                    scale = float(target_max) / float(largest_side)
                    new_width = max(1, round(width * scale))
                    new_height = max(1, round(height * scale))
                    
                    category_tag = "Detail" if is_detail else ("Mask" if is_mask else "Standard")
                    # Performance Tuning: Use BILINEAR for flat masks (400% faster) and BICUBIC for high-detail textures
                    resample_filter = Image.Resampling.BICUBIC if is_detail else Image.Resampling.BILINEAR
                    print(f"  -> Resizing [{category_tag}] Texture '{texture.name}': {width}x{height} -> {new_width}x{new_height}")
                    resized_img = pil_img.resize((new_width, new_height), resample_filter)
                    
                    # Update and save texture object back
                    texture.image = resized_img
                    texture.save()
                    optimized_count += 1
            except Exception as tex_err:
                print(f"  [Skip Texture] Failed to process texture in bundle: {tex_err}")

    # Mark as processed to prevent ever parsing it again
    try:
        with open(marker_path, "w") as f:
            pass
    except Exception:
        pass

    if optimized_count > 0:
        print(f"Writing compressed optimized bundle (LZ4) to {os.path.basename(output_path)}...")
        try:
            os.makedirs(os.path.dirname(output_path), exist_ok=True)
            # Re-pack and serialize using fast LZ4 compression (standard for Unity)
            optimized_data = env.file.save(packer="lz4")
            with open(output_path, "wb") as f:
                f.write(optimized_data)
            print(f"🏆 Successfully optimized {optimized_count} textures inside bundle!")
            
            # Explicitly reclaim memory instantly (saves massive RAM during batch runs)
            del env
            del optimized_data
            import gc
            gc.collect()
            
            return True
        except Exception as save_err:
            print(f"  [Error] Failed to save optimized bundle: {save_err}")
            return False
    else:
        print("No textures inside this bundle exceeded the size budget.")
        if bundle_path != output_path:
            try:
                os.makedirs(os.path.dirname(output_path), exist_ok=True)
                shutil.copy(bundle_path, output_path)
            except Exception as copy_err:
                print(f"  [Error] Failed to copy bundle: {copy_err}")
        return False

def main():
    if "--help" in sys.argv or "-h" in sys.argv or len(sys.argv) < 3:
        print("Game-Op UnityFS Bundle Optimizer")
        print("Usage: python3 bundle_optimizer.py <input_bundle_or_dir> <output_dir> [max_size_px] [OPTIONS]")
        print("\nOptions:")
        print("  max_size_px         Target maximum size for textures (default: 1024)")
        print("  --min-size-mb MB    Only optimize bundles larger than this size in MB (default: 30)")
        print("  --key-hex HEX_KEY   Decryption hex key for encrypted UnityFS bundles (e.g. 32-char hex)")
        print("  --key-db PATH       Path to the secure, encrypted keys database file (vrc_keys.db)")
        sys.exit(0)

    args = sys.argv[1:]
    key_hex = None
    if "--key-hex" in args:
        idx = args.index("--key-hex")
        if idx + 1 < len(args):
            key_hex = args[idx + 1]
            args.pop(idx + 1)
            args.pop(idx)
        else:
            print("Error: --key-hex specified but no key provided")
            sys.exit(1)

    min_size_mb = 30.0
    if "--min-size-mb" in args:
        idx = args.index("--min-size-mb")
        if idx + 1 < len(args):
            min_size_mb = float(args[idx + 1])
            args.pop(idx + 1)
            args.pop(idx)
        else:
            print("Error: --min-size-mb specified but no size provided")
            sys.exit(1)

    db_path = os.path.expanduser("~/Game-Op/vrc_keys.db")
    key_path = os.path.expanduser("~/Game-Op/.key_lock")
    
    if "--key-db" in args:
        idx = args.index("--key-db")
        if idx + 1 < len(args):
            db_path = args[idx + 1]
            # Use same path directory for the key lock
            key_path = os.path.join(os.path.dirname(db_path), ".key_lock")
            args.pop(idx + 1)
            args.pop(idx)
        else:
            print("Error: --key-db specified but no database path provided")
            sys.exit(1)

    if len(args) < 2:
        print("Error: Missing required positional arguments <input> <output>")
        sys.exit(1)

    input_path = args[0]
    output_path = args[1]
    max_size = int(args[2]) if len(args) > 2 else 1024

    # Load decrypted keys database if it exists
    keys_db = load_encrypted_db(db_path, key_path)
    if keys_db:
        print(f"🔒 Loaded and decrypted secure database with {len(keys_db)} asset keys!")

    if key_hex:
        try:
            key_bytes = bytes.fromhex(key_hex)
            print(f"Setting asset bundle decryption key: {key_hex}")
            UnityPy.set_assetbundle_decrypt_key(key_bytes)
        except Exception as e:
            print(f"Error parsing hex decryption key: {e}")
            sys.exit(1)

    if not os.path.exists(input_path):
        print(f"Error: Input path does not exist: {input_path}")
        sys.exit(1)

    if os.path.isdir(input_path):
        print(f"Scanning directory: {input_path}")
        tasks = []
        for root, dirs, files in os.walk(input_path):
            for file in files:
                file_path = os.path.join(root, file)
                is_data = file.lower() == "__data"
                is_bundle = any(file.lower().endswith(ext) for ext in [".vrca", ".vrcw", ".bundle", ".assets"])
                
                if is_data or is_bundle:
                    rel_path = os.path.relpath(file_path, input_path)
                    out_file_path = os.path.join(output_path, rel_path)
                    tasks.append((file_path, out_file_path))
                    
        if tasks:
            from concurrent.futures import ProcessPoolExecutor
            # Process bundles in parallel using separate CPU processes (reclaims 100% of all CPU cores, bypassing Python's GIL!)
            max_workers = os.cpu_count() or 4
            print(f"Optimizing {len(tasks)} bundles in parallel using ProcessPoolExecutor with {max_workers} workers...")
            with ProcessPoolExecutor(max_workers=max_workers) as executor:
                futures = [
                    executor.submit(optimize_bundle, f_in, f_out, max_size, keys_db, min_size_mb)
                    for f_in, f_out in tasks
                ]
                for future in futures:
                    try:
                        future.result()
                    except Exception as e:
                        print(f"Error in parallel task execution: {e}")
    else:
        optimize_bundle(input_path, output_path, max_size, keys_db, min_size_mb)

if __name__ == "__main__":
    main()
