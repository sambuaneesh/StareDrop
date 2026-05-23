# Security Notes

## Threat model (MVP)

- Passive visual observation is possible by nearby observers.
- Camera pipeline can produce corrupted/missing frames.
- Receiver must detect corruption and fail safely.

## Current (Phase 1)

- No encryption yet.
- Integrity checks exist at protocol utility level (CRC32 + SHA-256 helpers).
- No auto-open of received files.

## Planned (Phase 5)

- Compression before encryption.
- Authenticated encryption (ChaCha20-Poly1305 preferred).
- Argon2id password KDF with random salt.
- Clean failure on wrong password/auth tag mismatch.

## Non-goals

- Stealth transfer or hidden exfiltration behavior.
- Silent camera usage without user initiation.
