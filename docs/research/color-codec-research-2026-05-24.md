# Color Throughput Research Notes (May 24, 2026)

Scope: practical guidance for adding contrasting-color visual transfer modes to StareDrop.

## Sources reviewed

1. HiQ / robust color QR decoding:
   - Paper: "Robust and Fast Decoding of High-Capacity Color QR Codes for Mobile Applications" (arXiv:1704.06447)
   - URL: https://arxiv.org/abs/1704.06447
2. HiQ precursor (ICIP 2016):
   - Paper: "Towards Robust Color Recovery for High-Capacity Color QR Codes"
   - URL: https://personal.ie.cuhk.edu.hk/~ccloy/files/icip_2016_qr.pdf
3. Per-channel display color barcode framework:
   - Paper: "Per-Channel Color Barcodes for Displays" (ICIP 2016)
   - URL: https://hajim.rochester.edu/ece/sites/gsharma/papers/DineshPerChlDispColBarcodesICIP2016_07533109.pdf
4. TXQR protocol and testing ideas:
   - Repo: https://github.com/divan/txqr
   - License text (MIT): https://raw.githubusercontent.com/divan/txqr/master/LICENSE
5. Cimbar references and licensing constraints:
   - Repo: https://github.com/sz3/libcimbar
   - License text (MPL-2.0): https://raw.githubusercontent.com/sz3/libcimbar/master/LICENSE

## Core technical takeaways applied

1. Color helps capacity, but color decoding errors are dominated by:
   - cross-channel interference
   - cross-module interference
   - illumination variation
2. Layered/per-channel design is a strong baseline:
   - map multiple monochrome streams into color channels
   - keep decoder structure modular and recover channels separately
3. Calibration matters:
   - pilot/reference blocks or known symbols should estimate channel mixing
   - linear RGB mixing/cancellation is a practical first model for displays+camera
4. Geometry robustness matters more at higher density:
   - sample module centers
   - use more than just outer corners when estimating perspective transform

## Decisions for StareDrop (this iteration)

1. Added an experimental `color-grid` codec to simulation only (no camera pipeline yet).
2. Used a constrained, high-contrast 4-color palette (`BwRg`) as a low-risk first step.
3. Added benchmark schema fields that separate:
   - host processing throughput (`throughput_kib_s`)
   - modeled transfer throughput (`modeled_link_kib_s`)
4. Added persistent benchmark history append to:
   - `docs/research/benchmark-history.csv`

## Immediate next engineering steps

1. Add calibration cells and channel-mixing correction to color-grid decode.
2. Integrate color-grid into the camera path as experimental mode (Phase 9+).
3. Add loss/FEC strategy comparison for color-grid vs QR under equal noise profiles.
4. Add standardized lighting and angle test matrix for color mode regression.
