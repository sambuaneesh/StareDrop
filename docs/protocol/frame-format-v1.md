# Frame Format V1 (MVP JSON)

## Magic and version

- `magic`: `"STAREDROP"`
- `version`: `1`

## Frame types

- `MANIFEST`
- `DATA`
- `CONTROL`
- `END`
- `TEXT` (Phase 1 helper frame for static QR demo)

## JSON manifest frame

```json
{
  "magic": "STAREDROP",
  "version": 1,
  "frame_type": "MANIFEST",
  "session_id": "uuid",
  "file_id": "uuid",
  "file_name": "example.txt",
  "mime_type": "text/plain",
  "original_file_size": 1234,
  "processed_file_size": 1234,
  "chunk_size": 700,
  "total_chunks": 2,
  "compression": "none",
  "encryption": "none",
  "original_sha256": "hex",
  "processed_sha256": "hex"
}
```

## JSON data frame

```json
{
  "magic": "STAREDROP",
  "version": 1,
  "frame_type": "DATA",
  "session_id": "uuid",
  "file_id": "uuid",
  "file_name": "example.txt",
  "file_size": 1234,
  "chunk_index": 0,
  "total_chunks": 2,
  "payload_base64": "....",
  "crc32": 123456789
}
```

## JSON control frame

```json
{
  "magic": "STAREDROP",
  "version": 1,
  "frame_type": "CONTROL",
  "session_id": "uuid",
  "control_type": "MISSING_CHUNKS",
  "missing_chunks": [1, 3, 9]
}
```

## Checksum rules

- Per-frame payload CRC32 for early corruption rejection.
- End-to-end SHA-256 verification after reassembly.

## Future extension: Color/contrast codec profile (planned)

This is not active in V1, but reserved for future high-throughput modes.

Proposed manifest additions (future version):

- `visual_codec`: `qr` | `grid_mono` | `grid_color`
- `color_profile`: `none` | `bw` | `bw_rg` | `bw_rgb`
- `cell_bits`: bits represented by each sampled cell/symbol
- `calibration_required`: bool
- `calibration_frame_interval`: number of frames between calibration refresh

Proposed frame-level additions (future version):

- `palette_id`: sender-selected palette identifier
- `frame_exposure_hint`: optional normalization hint
- `color_checksum`: checksum across quantized symbol channels

Compatibility rule:

- Receiver must reject unknown active `visual_codec` modes unless explicitly supported.
- Sender must provide a fallback profile (`qr` or `grid_mono`) when operating in mixed-capability environments.
