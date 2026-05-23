# Benchmark Metrics (Planned)

Phase 4 will log per-session metrics:

- file name and size
- codec/mode/FPS/chunk size
- frames displayed/captured/decoded
- invalid frames and duplicate chunks
- time to first decode
- time to completion
- effective throughput (KB/s)
- final hash verification result

## Current

- Only helper metric function exists in `staredrop-benchmark`.
- Benchmark UI/export is not implemented yet.
