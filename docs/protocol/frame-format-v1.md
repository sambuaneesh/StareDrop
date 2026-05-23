# Frame Format V1 (MVP JSON)

## Magic and version

- `magic`: `"OPTIGAP"`
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
  "magic": "OPTIGAP",
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
  "magic": "OPTIGAP",
  "version": 1,
  "frame_type": "DATA",
  "session_id": "uuid",
  "file_id": "uuid",
  "chunk_index": 0,
  "total_chunks": 2,
  "payload_base64": "....",
  "crc32": 123456789
}
```

## JSON control frame

```json
{
  "magic": "OPTIGAP",
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
