# StareDrop Phase 2 Manual Checklist

Date: __________  
Tester: __________  
Sender device: __________  
Receiver device: __________  
Camera index used: __________

## Test artifacts

- `manual-tests/phase2/sample-1kb.bin`
- `manual-tests/phase2/sample-10kb.bin`
- `manual-tests/phase2/sample-100kb.bin`
- `manual-tests/phase2/sample.txt`
- Expected hashes: `manual-tests/phase2/SHA256SUMS.txt`

## Common commands

List cameras:

```bash
cargo run -p staredrop-app -- list-cameras
```

Receiver (recommended baseline):

```bash
mkdir -p manual-tests/phase2/received
cargo run -p staredrop-app -- receiver --camera-index 0 --auto-start --output-dir manual-tests/phase2/received --print-decoded false --auto-save true
```

Sender pattern:

```bash
cargo run -p staredrop-app -- sender --send-file <PATH> --chunk-size <N> --fps <FPS>
```

Hash verify output:

```bash
sha256sum manual-tests/phase2/received/<OUTPUT_FILE>
```

## Cases

### Case A: 1KB baseline

Sender:

```bash
cargo run -p staredrop-app -- sender --send-file manual-tests/phase2/sample-1kb.bin --chunk-size 700 --fps 8
```

Pass criteria:

- Receiver saves file.
- Hash matches `sample-1kb.bin`.

Result: PASS / FAIL  
Notes: ______________________

### Case B: 10KB baseline

Sender:

```bash
cargo run -p staredrop-app -- sender --send-file manual-tests/phase2/sample-10kb.bin --chunk-size 700 --fps 8
```

Pass criteria:

- Receiver saves file.
- Hash matches `sample-10kb.bin`.

Result: PASS / FAIL  
Notes: ______________________

### Case C: 100KB baseline

Sender:

```bash
cargo run -p staredrop-app -- sender --send-file manual-tests/phase2/sample-100kb.bin --chunk-size 700 --fps 8
```

Pass criteria:

- Receiver saves file.
- Hash matches `sample-100kb.bin`.

Result: PASS / FAIL  
Notes: ______________________

### Case D: Late receiver start

Procedure:

1. Start sender first with 10KB case.
2. Wait ~10 seconds.
3. Start receiver.

Pass criteria:

- Transfer still completes.
- Hash matches `sample-10kb.bin`.

Result: PASS / FAIL  
Notes: ______________________

### Case E: Throughput profile variation

Run 10KB case with:

1. `--chunk-size 400 --fps 6`
2. `--chunk-size 700 --fps 8`
3. `--chunk-size 1000 --fps 10`

Pass criteria:

- At least one profile consistently completes quickly.
- No corrupted output saves.

Result: PASS / FAIL  
Notes: ______________________
