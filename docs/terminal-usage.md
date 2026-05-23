# StareDrop Terminal Usage (Phase 1)

StareDrop is CLI-driven in Phase 1. The terminal selects mode and options; the desktop window is used for fullscreen QR display or camera scanning.

## Command shape

```bash
cargo run -p staredrop-app -- [global-flags] <subcommand> [subcommand-flags]
```

## Subcommands

1. `list-cameras`
2. `sender`
3. `receiver`

## Global flags

1. `--fullscreen <true|false>` (default: `true`)
2. `--overlay <true|false>` (default: `true`)

## Sender flags

1. `--text <TEXT>`: inline payload text.
2. `--input-file <PATH>`: payload from file.
3. `--input-format <utf8|base64>` (default: `utf8`):
   - `utf8`: file bytes must be valid UTF-8.
   - `base64`: file bytes encoded as Base64 text for QR payload.

Rules:

1. Use exactly one of `--text` or `--input-file`.
2. If file is binary, use `--input-format base64`.

Examples:

```bash
cargo run -p staredrop-app -- sender --text "hello world"
cargo run -p staredrop-app -- sender --input-file ./payload.txt --input-format utf8
cargo run -p staredrop-app -- sender --input-file ./sample.bin --input-format base64
```

## Receiver flags

1. `--camera-index <N>` (default: `0`)
2. `--auto-start` (default: `false`)
3. `--print-decoded <true|false>` (default: `true`)

Examples:

```bash
cargo run -p staredrop-app -- list-cameras
cargo run -p staredrop-app -- receiver --camera-index 0
cargo run -p staredrop-app -- receiver --camera-index 1 --auto-start
```

## Receiver keyboard controls

1. `Space`: start/stop scanning
2. `R`: refresh camera list
3. `Q` or `Esc`: quit app

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

## Scope in Phase 1

This mode currently supports static QR text transfer only. Multi-frame file transfer starts in Phase 2.
