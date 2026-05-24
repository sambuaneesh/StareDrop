# StareDrop MVP Roadmap

## Current phase status

- Phase 0: **Completed**
- Phase 1: **Completed**
- Phase 2: **In progress (animated QR file transfer MVP implemented)**
- Phase 2: **In progress (animated QR file transfer + simulation benchmark harness implemented)**
- Phase 3+: **Not started**

## Phase 0 checklist

- [x] Cargo workspace created
- [x] Modular crate layout created
- [x] `eframe/egui` app skeleton created
- [x] Docs scaffold created
- [x] Existing projects reviewed with license notes
- [x] Core protocol/chunking utility tests added
- [ ] CI config (pending)

## Phase 1 checklist

- [x] Terminal-driven sender mode
- [x] QR encoding (`qrcode`)
- [x] Fullscreen QR display surface
- [x] Terminal-driven receiver mode
- [x] Camera capture loop (`rscam`)
- [x] QR decode loop (`rqrr`)
- [x] Fullscreen camera scan surface
- [x] Decoded text in overlay + terminal output
- [x] Basic logging setup

## Phase 2 preview

Next milestone is small file transfer over animated QR:

1. [x] manifest/data JSON frames
2. [x] chunking + reassembly in app flow
3. [x] repeated frame animation
4. [x] integrity verification and output save flow
5. [ ] manual two-device reliability validation matrix

## Experimental progress (pre-Phase 9)

- [x] Added simulation-only `color-grid` codec path for throughput experiments.
- [x] Added persistent benchmark history logging (`docs/research/benchmark-history.csv`).
- [ ] Add camera-path color-grid decode with calibration/correction pipeline.

## Future throughput track (planned)

Goal: increase effective data throughput by encoding more bits per visual cell using high-contrast multi-color symbols.

Planned direction:

1. Add a new `ColorGrid` experimental codec profile after monochrome grid stability.
2. Use a constrained, high-contrast palette first (for example black/white + 2 accent colors), then expand only if error rates remain acceptable.
3. Introduce calibration frames before payload frames:
   - white balance reference
   - per-channel gain normalization
   - brightness/exposure sanity check
4. Keep codec modular under `VisualEncoder`/`VisualDecoder` traits so QR mode remains available as fallback.
5. Add adaptive density/palette mode switching:
   - `Safe`: monochrome only
   - `Balanced`: limited color symbols
   - `Fast`: higher color symbol density
6. Require benchmark evidence before defaulting to color mode:
   - higher effective KiB/s than monochrome/QR
   - acceptable frame decode success rate in varied lighting
   - no regression in end-to-end integrity success

Risks to address in this track:

- display color profile variability across devices
- camera auto-exposure and white-balance drift
- compression and sensor noise causing channel cross-talk
- reduced robustness at oblique viewing angles
