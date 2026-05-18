# Firmware Boundary

Do not commit firmware images.

Expected local source image:

```text
firmware/resmed-air10.bin
```

Alternative:

```bash
AIRBREAK_SOURCE_FIRMWARE=/private/path/resmed-air10.bin ./scripts/run-station-pipeline.sh
```

Generated patched images are written under `artifacts/firmware/` and are also ignored by git.
When a candidate is worth preserving, record the command, source SHA-256, patched SHA-256,
patch log, and emulator evidence path in notes or a commit message, not as committed binaries.
