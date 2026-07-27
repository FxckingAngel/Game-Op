# Game-Op

Game-Op stages and optimizes game assets into the directory selected with `--asset-output`.

## Unity asset bundles

Game-Op includes a safe, read-only Unity asset-bundle-aware path for VRChat caches and normal Unity bundle files. The original bundle is never overwritten. When `AssetStager` detects a bundle, it inspects the file and writes a report under `--asset-output/unity_bundles/` instead of modifying the source bundle.

Supported detection:

- Unity bundle signatures: `UnityFS`, `UnityWeb`, and `UnityRaw`.
- Common bundle extensions: `.vrca`, `.bundle`, `.unity3d`, and `.ab`.
- Common VRChat cache paths, including `VRChat/VRChat/Cache-WindowsPlayer` and `AppData/LocalLow/VRChat/VRChat/Cache`.

Inspection is intentionally conservative and dependency-free: Game-Op scans bundle metadata-like bytes for asset names and lists contained textures, meshes, materials, shaders, and audio by common file extensions. This mock-parser path is suitable for fixture tests and for deciding whether a file should be handled as a Unity bundle, but it does not decompress or rewrite proprietary Unity bundle payloads.

Bundles that do not match a Unity signature, supported extension, or known VRChat cache path are passed through the normal staging path unchanged. Unity bundle payloads whose internal records cannot be recognized are also passed through without extracted optimized assets; only an inspection report is written.
