# Benchmark Results (May 24, 2026)

Noise profile used in all runs:

- `--loops 3`
- `--drop-every 9`
- `--corrupt-every 23`
- `--reverse-data-order true`
- input file: `manual-tests/phase2/sample-100kb.bin` (100 KiB)

## Results snapshot

| Run label | Codec | Chunk size | Grid side | Completion ms | Modeled display ms | Host throughput (KiB/s) | Modeled link (KiB/s) | Complete | SHA match |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `qr-noise-baseline-v2` | `qr` | 1000 | n/a | 93197.898 | 26000.000 | 1.0730 | 3.8462 | true | true |
| `color-grid-noise-exp-v2` | `color-grid` | 1000 | 96 | 7965.894 | 26000.000 | 12.5535 | 3.8462 | true | true |
| `color-grid-c1800-exp-v2` | `color-grid` | 1800 | 128 | 5228.726 | 14500.000 | 19.1251 | 6.8966 | true | true |

Source of record:

- `docs/research/benchmark-history.csv`

## Interpretation

1. At equal chunking/frame-count (`chunk=1000`), color-grid improves host encode/decode speed but modeled link throughput is unchanged.
2. Color-grid permits a larger chunk size (`1800`) under this simulation path where QR failed to encode at the same chunk size.
3. Larger chunking with color-grid reduced total frame count and improved modeled link throughput from `3.8462` to `6.8966` KiB/s (about `1.79x`).

## Reproduction commands

```bash
cargo run -p staredrop-app -- simulate \
  --input-file manual-tests/phase2/sample-100kb.bin \
  --visual-codec qr \
  --chunk-size 1000 --fps 12 \
  --loops 3 --drop-every 9 --corrupt-every 23 \
  --reverse-data-order true \
  --run-label qr-noise-baseline-v2 \
  --output-dir manual-tests/bench-sweeps/2026-05-24-color/qr-c1000-f12-v2

cargo run -p staredrop-app -- simulate \
  --input-file manual-tests/phase2/sample-100kb.bin \
  --visual-codec color-grid \
  --grid-side 96 --cell-pixels 8 --quiet-zone-cells 2 \
  --chunk-size 1000 --fps 12 \
  --loops 3 --drop-every 9 --corrupt-every 23 \
  --reverse-data-order true \
  --run-label color-grid-noise-exp-v2 \
  --output-dir manual-tests/bench-sweeps/2026-05-24-color/color-grid-c1000-f12-v2

cargo run -p staredrop-app -- simulate \
  --input-file manual-tests/phase2/sample-100kb.bin \
  --visual-codec color-grid \
  --grid-side 128 --cell-pixels 8 --quiet-zone-cells 2 \
  --chunk-size 1800 --fps 12 \
  --loops 3 --drop-every 9 --corrupt-every 23 \
  --reverse-data-order true \
  --run-label color-grid-c1800-exp-v2 \
  --output-dir manual-tests/bench-sweeps/2026-05-24-color/color-grid-c1800-f12-v2
```
