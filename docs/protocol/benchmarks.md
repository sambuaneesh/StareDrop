# Benchmark Metrics

Phase 2 includes a terminal-only simulation benchmark path (`simulate`) that runs sender->QR encode->QR decode->receiver reassembly without camera hardware.

## Run

```bash
# default suite
cargo run -p staredrop-app -- simulate

# custom file
cargo run -p staredrop-app -- simulate --input-file ./payload.bin --output-dir ./manual-tests/sim-output
```

## Exported artifacts

- `<output-dir>/simulation-summary.csv`
- `<output-dir>/simulation-summary.txt`
- `<output-dir>/received/...` (only when completion succeeds)

## Current metrics per case

- `bytes_in`, `bytes_out`
- `chunk_size`, `total_chunks`
- `frames_in_plan`, `loops`, `frames_generated`
- `frames_dropped` (simulated)
- `frames_encoded`, `frames_decoded`, `decode_failures`
- `accepted_chunks`, `duplicate_chunks`, `invalid_chunks`
- `completed`, `sha_match`, `byte_diff`
- `total_ms`, `completion_ms`, `encode_ms`, `decode_ms`
- `modeled_display_ms` (from `fps`)
- `throughput_kib_s`
- `compression_ratio` (currently `1.0` in Phase 2)
- `protocol_overhead_ratio`

## Notes

- Loss/corruption knobs (`drop-every`, `corrupt-every`) are useful for Phase 2 stress testing.
- Without FEC/retransmit strategy, lossy runs may fail to complete (expected until later phases).

## Future metrics for contrasting-color throughput work

When color-based codec modes are introduced, add:

- `palette_profile` (which contrast palette was used)
- per-channel symbol error rate (R/G/B or selected channels)
- calibration success/failure count
- post-calibration decode uplift vs no-calibration baseline
- throughput delta vs monochrome baseline at same noise profile
- completion success rate by lighting profile (dark/normal/bright)
- completion success rate by viewing angle bucket

Acceptance target for enabling color by default:

- statistically higher median throughput than monochrome baseline
- no material reduction in file reconstruction integrity success
