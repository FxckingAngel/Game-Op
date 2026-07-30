#!/usr/bin/env python3
# ==================================================================
# Game-Op Live Validation Harness  --  RUN THIS ON YOUR TARGET MACHINE
# ==================================================================
# What it does:
#   1. Finds your VRChat cache (or use --cache PATH).
#   2. Copies it (or the N largest bundles) into a temp workdir -- your REAL
#      cache is never modified.
#   3. Runs bundle_optimizer.py on the copy and measures the REAL before/after
#      size of every bundle. That on-disk reduction is the direct, honest proxy
#      for VRAM savings (a texture that shrinks on disk loads smaller into VRAM).
#   4. Writes game-op-validation-report.txt that you can send back.
#
# What it does NOT do (needs the game actually running -- manual step, printed
# at the end): measure live in-game VRAM/FPS on your Intel iGPU.
#
# Safe to run as a normal user. Nothing here writes to your real cache.
# ==================================================================
import argparse
import json
import os
import platform
import shutil
import subprocess
import sys
import tempfile
import time

BUNDLE_EXTS = (".vrca", ".vrcw", ".bundle", ".assets", ".unity3d")


def human(n):
    for unit in ("B", "KB", "MB", "GB", "TB"):
        if abs(n) < 1024.0:
            return f"{n:.1f} {unit}"
        n /= 1024.0
    return f"{n:.1f} PB"


def is_bundle(fname):
    low = fname.lower()
    return low == "__data" or low.endswith(BUNDLE_EXTS)


def collect_bundles(root):
    """Return list of (relpath, size_bytes) for every bundle under root."""
    out = []
    for r, _, files in os.walk(root):
        for f in files:
            if is_bundle(f):
                full = os.path.join(r, f)
                try:
                    out.append((os.path.relpath(full, root), os.path.getsize(full)))
                except OSError:
                    pass
    return out


def default_cache_candidates():
    """Cross-platform VRChat cache directory candidates."""
    home = os.path.expanduser("~")
    cands = []
    if os.name == "nt":
        # Native Windows VRChat cache.
        local_low = os.path.join(home, "AppData", "LocalLow", "VRChat", "VRChat")
        cands += [os.path.join(local_low, "Cache-WindowsPlayer"), local_low]
    else:
        # Linux + Proton/Flatpak (Steam appid 438100) and native Linux builds.
        proton_tail = os.path.join(
            "steamapps", "compatdata", "438100", "pfx", "drive_c", "users",
            "steamuser", "AppData", "LocalLow", "VRChat", "VRChat",
        )
        for steam_root in (
            "~/.steam/steam", "~/.local/share/Steam", "~/.steam/root",
            "~/.var/app/com.valvesoftware.Steam/.local/share/Steam",
        ):
            base = os.path.join(os.path.expanduser(steam_root), proton_tail)
            cands += [os.path.join(base, "Cache-WindowsPlayer"), base]
        # Native Linux VRChat (rare) LocalLow-style path.
        cands.append(os.path.join(home, ".config", "unity3d", "VRChat", "VRChat"))
    return cands


def find_cache():
    for c in default_cache_candidates():
        if os.path.isdir(c) and collect_bundles(c):
            return c
    return None


def detect_gpu():
    try:
        if os.name == "nt":
            out = subprocess.run(
                ["powershell", "-NoProfile", "-Command",
                 "Get-CimInstance Win32_VideoController | Select-Object -ExpandProperty Name"],
                capture_output=True, text=True, timeout=15).stdout
        elif sys.platform == "darwin":
            out = subprocess.run(
                ["sh", "-c", "system_profiler SPDisplaysDataType | awk -F': ' '/Chipset Model/{print $2}'"],
                capture_output=True, text=True, timeout=15).stdout
        else:
            out = subprocess.run(
                ["sh", "-c", "lspci 2>/dev/null | grep -Ei 'vga|3d|display' || true"],
                capture_output=True, text=True, timeout=15).stdout
        names = [l.strip() for l in out.splitlines() if l.strip()]
        return ", ".join(names) if names else "unknown"
    except Exception:
        return "unknown"


def sample_intel_vram():
    """Best-effort single intel_gpu_top JSON sample (Linux+Intel only)."""
    if os.name == "nt" or sys.platform == "darwin" or not shutil.which("intel_gpu_top"):
        return None
    try:
        # One ~1s sample, JSON output. Requires permission on some systems.
        proc = subprocess.run(["intel_gpu_top", "-J", "-s", "1000", "-o", "-"],
                              capture_output=True, text=True, timeout=8)
        # intel_gpu_top -J streams JSON objects; grab the first complete one.
        txt = proc.stdout.strip()
        if not txt:
            return None
        # Trim to the first balanced {...} block.
        depth = 0
        for i, ch in enumerate(txt):
            if ch == "{":
                depth += 1
            elif ch == "}":
                depth -= 1
                if depth == 0:
                    return json.loads(txt[: i + 1])
    except Exception:
        return None
    return None


def find_optimizer():
    here = os.path.dirname(os.path.abspath(__file__))
    cand = os.path.join(here, "bundle_optimizer.py")
    return cand if os.path.exists(cand) else None


def main():
    ap = argparse.ArgumentParser(description="Game-Op on-laptop validation harness")
    ap.add_argument("--cache", help="Path to VRChat cache (auto-detected if omitted)")
    ap.add_argument("--sample", type=int, default=0,
                    help="Only test the N largest bundles (0 = all). Faster for a quick check.")
    ap.add_argument("--max-size", type=int, default=1024, help="Texture budget px (default 1024)")
    ap.add_argument("--generic", action="store_true",
                    help="Optimize as a generic Unity game (skip the VRChat key pipeline)")
    ap.add_argument("--report", default="game-op-validation-report.txt")
    ap.add_argument("--keep", action="store_true", help="Keep the temp workdir for inspection")
    args = ap.parse_args()

    optimizer = find_optimizer()
    if not optimizer:
        print("ERROR: bundle_optimizer.py not found next to this script. Run it from your Game-Op folder.")
        sys.exit(1)

    cache = args.cache or find_cache()
    if not cache or not os.path.isdir(cache):
        print("ERROR: could not find a VRChat cache. Pass --cache /path/to/Cache-WindowsPlayer")
        print("Searched:")
        for c in default_cache_candidates():
            print(f"  - {c}")
        sys.exit(1)

    before = collect_bundles(cache)
    if not before:
        print(f"ERROR: no bundles found under {cache}")
        sys.exit(1)
    before.sort(key=lambda t: t[1], reverse=True)
    if args.sample and args.sample > 0:
        before = before[: args.sample]

    print(f"Cache: {cache}")
    print(f"Bundles to test: {len(before)}  ({human(sum(s for _, s in before))})")

    workdir = tempfile.mkdtemp(prefix="game-op-validate-")
    src_copy = os.path.join(workdir, "in")
    out_dir = os.path.join(workdir, "out")
    os.makedirs(src_copy, exist_ok=True)

    # Copy only the bundles we are testing (plus each one's __info companion, which
    # the VRChat key pipeline needs). Real cache stays untouched.
    print("Copying bundles into a safe temp workdir (your real cache is not modified)...")
    for rel, _ in before:
        s = os.path.join(cache, rel)
        d = os.path.join(src_copy, rel)
        os.makedirs(os.path.dirname(d), exist_ok=True)
        try:
            shutil.copy2(s, d)
            info = os.path.join(os.path.dirname(s), "__info")
            if os.path.exists(info):
                shutil.copy2(info, os.path.join(os.path.dirname(d), "__info"))
        except OSError as e:
            print(f"  [skip] {rel}: {e}")

    cmd = [sys.executable, optimizer, src_copy, out_dir, str(args.max_size), "--min-size-mb", "0"]
    if args.generic:
        cmd.append("--generic")
    print(f"Running optimizer: {' '.join(cmd)}")
    t0 = time.time()
    proc = subprocess.run(cmd, capture_output=True, text=True)
    elapsed = time.time() - t0
    opt_log = (proc.stdout or "") + (proc.stderr or "")

    # Honest per-bundle pairing: if the optimizer produced an output file use its
    # size; if not (unchanged / couldn't decrypt), the bundle counts unchanged.
    rows, total_before, total_after, shrunk = [], 0, 0, 0
    for rel, bsize in before:
        outp = os.path.join(out_dir, rel)
        asize = os.path.getsize(outp) if os.path.exists(outp) else bsize
        total_before += bsize
        total_after += asize
        if asize < bsize:
            shrunk += 1
        rows.append((rel, bsize, asize))

    rows.sort(key=lambda r: (r[1] - r[2]), reverse=True)
    saved = total_before - total_after
    pct = (saved / total_before * 100.0) if total_before else 0.0

    gpu = detect_gpu()
    vram = sample_intel_vram()

    # ---- write report ----
    lines = []
    lines.append("Game-Op Validation Report")
    lines.append("=" * 60)
    lines.append(f"Date            : {time.strftime('%Y-%m-%d %H:%M:%S')}")
    lines.append(f"OS / platform   : {platform.platform()}")
    lines.append(f"Python          : {platform.python_version()}")
    lines.append(f"CPU cores       : {os.cpu_count()}")
    lines.append(f"GPU             : {gpu}")
    lines.append(f"Mode            : {'generic Unity' if args.generic else 'VRChat'}")
    lines.append(f"Texture budget  : {args.max_size}px")
    lines.append(f"Cache path      : {cache}")
    lines.append(f"Optimizer time  : {elapsed:.1f}s for {len(before)} bundles")
    lines.append("")
    lines.append("ASSET SIZE (direct proxy for VRAM load)")
    lines.append("-" * 60)
    lines.append(f"Bundles tested  : {len(before)}")
    lines.append(f"Bundles shrunk  : {shrunk}")
    lines.append(f"Total before    : {human(total_before)}")
    lines.append(f"Total after     : {human(total_after)}")
    lines.append(f"Saved           : {human(saved)}  ({pct:.1f}%)")
    lines.append("")
    lines.append("Top 15 reductions:")
    for rel, b, a in rows[:15]:
        d = b - a
        p = (d / b * 100.0) if b else 0.0
        tag = os.path.basename(os.path.dirname(rel)) or rel
        lines.append(f"  {human(b):>10} -> {human(a):>10}  (-{p:4.1f}%)  {tag}")
    if vram is not None:
        lines.append("")
        lines.append("intel_gpu_top sample (raw JSON, idle unless VRChat is running):")
        lines.append("  " + json.dumps(vram)[:500])
    lines.append("")
    lines.append("OPTIMIZER LOG (tail)")
    lines.append("-" * 60)
    lines.extend(opt_log.strip().splitlines()[-40:])
    lines.append("")
    lines.append("MANUAL STEP -- live in-game VRAM/FPS (only you can do this)")
    lines.append("-" * 60)
    lines.append("1. Close VRChat. In a terminal run:  intel_gpu_top")
    lines.append("2. Launch VRChat via ./start_vrc.sh and load your usual busy world/avatars.")
    lines.append("3. Note peak 'used' memory + your FPS. This is the BASELINE.")
    lines.append("4. Let Game-Op optimize the cache (start_vrc.sh does this live), rejoin,")
    lines.append("   and record VRAM + FPS again. Paste both numbers back with this report.")

    report = "\n".join(lines) + "\n"
    with open(args.report, "w") as f:
        f.write(report)

    print("\n" + report)
    print(f"Report written to: {os.path.abspath(args.report)}")

    if args.keep:
        print(f"Temp workdir kept at: {workdir}")
    else:
        shutil.rmtree(workdir, ignore_errors=True)


if __name__ == "__main__":
    main()
