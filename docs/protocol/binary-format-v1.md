# Binary Format V1 (Planned)

Binary mode is planned for Phase 6 to reduce JSON/Base64 overhead.

## Header layout (little-endian)

- `magic`: 4 bytes (`OPTG`)
- `version`: 1 byte
- `frame_type`: 1 byte
- `flags`: 2 bytes
- `session_id`: 16 bytes
- `file_id`: 16 bytes
- `chunk_index`: 4 bytes
- `total_chunks`: 4 bytes
- `payload_length`: 4 bytes
- `header_crc32`: 4 bytes
- `payload`: variable
- `payload_crc32`: 4 bytes

## Design notes

- Keep parsing deterministic and bounds-checked.
- Reject unknown frame types.
- Preserve room for future flags (compression/encryption/FEC markers).
