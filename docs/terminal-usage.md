# StareDrop Terminal Usage (Phase 2)

StareDrop is CLI-driven. The terminal selects mode and options; the desktop window is used for fullscreen visual frame display or camera scanning.

## Command shape

```bash
cargo run -p staredrop-app -- [global-flags] <subcommand> [subcommand-flags]
```

## Subcommands

1. `list-cameras`
2. `sender`
3. `receiver`
4. `simulate`

## Global flags

1. `--fullscreen <true|false>` (default: `true`)
2. `--overlay <true|false>` (default: `true`)

## Sender flags

1. `--text <TEXT>`: inline payload text (static QR mode).
2. `--input-file <PATH>`: static QR payload from file.
3. `--send-file <PATH>`: Phase 2 animated file-transfer source file.
4. `--input-format <utf8|base64>` (default: `utf8`) for `--input-file`.
5. `--chunk-size <N>` for `--send-file` (optional).
6. `--fps <N>` (default: `8`) for sender frame animation.
7. `--visual-codec <qr|color-grid>` (default: `qr`)
8. `--pixel-size <N>` (default: `8`) for color-grid

Rules:

1. Use exactly one of `--text`, `--input-file`, or `--send-file`.
2. If using `--input-file` with binary content, use `--input-format base64`.
3. In `--visual-codec color-grid` mode, StareDrop derives grid side from fullscreen size and `--pixel-size`.
4. In `--visual-codec color-grid` mode, omitting `--chunk-size` auto-selects near-max payload utilization for the computed grid settings.

Examples:

```bash
cargo run -p staredrop-app -- sender --text "hello world"
cargo run -p staredrop-app -- sender --input-file ./payload.txt --input-format utf8
cargo run -p staredrop-app -- sender --input-file ./sample.bin --input-format base64
cargo run -p staredrop-app -- sender --send-file ./payload.bin --chunk-size 700 --fps 8
cargo run -p staredrop-app -- sender --send-file ./payload.bin --visual-codec color-grid --pixel-size 8 --fps 12
```

## Receiver flags

1. `--camera-index <N>` (default: `0`)
2. `--auto-start` (default: `false`)
3. `--print-decoded <true|false>` (default: `false`)
4. `--output-file <PATH>`: exact output path (must not exist)
5. `--output-dir <PATH>` (default: `.`): output directory when output-file is not set
6. `--auto-save <true|false>` (default: `true`)
7. `--visual-codec <qr|color-grid>` (default: `qr`)
8. `--pixel-size <N>` (default: `8`) for color-grid

Examples:

```bash
cargo run -p staredrop-app -- list-cameras
cargo run -p staredrop-app -- receiver --camera-index 0
cargo run -p staredrop-app -- receiver --camera-index 1 --auto-start --output-dir ./received
cargo run -p staredrop-app -- receiver --camera-index 1 --output-file ./received/output.bin
cargo run -p staredrop-app -- receiver --camera-index 1 --auto-start --visual-codec color-grid --pixel-size 8 --output-dir ./received
```

## Receiver keyboard controls

1. `Space`: start/stop scanning
2. `R`: refresh camera list
3. `S`: manual save (useful with `--auto-save false`)
4. `Q` or `Esc`: quit app

## Simulate flags (camera-free benchmark mode)

1. `--input-file <PATH>` (repeatable): file(s) to simulate. If omitted, default suite is generated.
2. `--output-dir <PATH>` (default: `manual-tests/sim-output`)
3. `--chunk-size <N>` (default: `700`)
4. `--fps <N>` (default: `8`) modeled display FPS for time reporting
5. `--loops <N>` (default: `1`) repeat full sender frame cycle N times
6. `--reverse-data-order <true|false>` (default: `false`)
7. `--drop-every <N>` (default: `0`) drop every Nth DATA frame before decode
8. `--corrupt-every <N>` (default: `0`) corrupt every Nth DATA frame before encode
9. `--visual-codec <qr|color-grid>` (default: `qr`)
10. `--grid-side <N>` (default: `96`) color-grid side length in cells
11. `--cell-pixels <N>` (default: `8`) color-grid cell size
12. `--quiet-zone-cells <N>` (default: `2`) color-grid quiet-zone
13. `--record-history <true|false>` (default: `true`)
14. `--history-file <PATH>` (default: `docs/research/benchmark-history.csv`)
15. `--run-label <TEXT>` optional label for history rows

Examples:

```bash
cargo run -p staredrop-app -- simulate

cargo run -p staredrop-app -- simulate \
  --input-file ./manual-tests/phase2/sample-100kb.bin \
  --loops 2 \
  --drop-every 9 \
  --corrupt-every 17 \
  --reverse-data-order true \
  --output-dir ./manual-tests/sim-output-lossy

cargo run -p staredrop-app -- simulate \
  --input-file ./manual-tests/phase2/sample-100kb.bin \
  --visual-codec color-grid \
  --grid-side 128 \
  --cell-pixels 8 \
  --chunk-size 1800 \
  --fps 12 \
  --loops 3 \
  --drop-every 9 \
  --corrupt-every 23 \
  --reverse-data-order true \
  --run-label color-grid-c1800-exp \
  --output-dir ./manual-tests/bench-sweeps/2026-05-24-color/color-grid-c1800-f12
```

Simulation writes:

1. `<output-dir>/received/...` reconstructed files
2. `<output-dir>/simulation-summary.csv`
3. `<output-dir>/simulation-summary.txt`

## Linux / WSL notes

1. If Wayland startup fails, force X11:

```bash
STAREDROP_FORCE_X11=1 cargo run -p staredrop-app -- receiver --camera-index 0
```

2. If X11 fallback reports missing `libxkbcommon-x11.so`, install runtime:

```bash
sudo pacman -S --needed libxkbcommon-x11
```

3. On WSLg, ensure runtime env is valid:

```bash
export XDG_RUNTIME_DIR=/mnt/wslg/runtime-dir
export WAYLAND_DISPLAY=wayland-0
export DISPLAY=:0
```

## Scope in Phase 2

Phase 2 supports static text mode and animated multi-frame file transfer using JSON/Base64 frames with CRC32 and SHA-256 validation.
