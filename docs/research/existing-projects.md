# Existing Project Review (Phase 0)

Reviewed on **May 23, 2026**.

License checks were verified from GitHub repository metadata/API on this date.

## 1) ShadowCat

- Project: ShadowCat
- Repo: https://github.com/unprovable/ShadowCat
- Language: HTML/JavaScript (single-file browser tool)
- License: **No license detected** (`license: null`, no `/license` endpoint result)
- What it does: screen-to-camera animated QR transfer in browser
- Can we reuse code?: **No** (no explicit OSS license; treat as all-rights-reserved)
- Ideas to borrow:
  - UX for sender/receiver split
  - missing chunk/status visualization
  - tunable chunk size/FPS/ECC controls
- Risks/limitations:
  - browser runtime assumptions
  - no explicit reuse rights
- Rust-only conflict?: **Yes** for direct code reuse; **No** for concept-level inspiration

## 2) TXQR

- Project: TXQR
- Repo: https://github.com/divan/txqr
- Language: Go
- License: MIT
- What it does: animated QR transfer with fountain-code redundancy
- Can we reuse code?: **No direct code copy** (different language); protocol ideas can be reimplemented
- Ideas to borrow:
  - fountain/redundancy strategy
  - late receiver start tolerance
  - transfer framing patterns for animated codes
- Risks/limitations:
  - requires careful Rust reimplementation and validation
  - performance characteristics may differ by camera/decoder stack
- Rust-only conflict?: **No** for design inspiration; **Yes** for direct implementation dependency

## 3) libcimbar / Cimbar

- Project: libcimbar
- Repo: https://github.com/sz3/libcimbar
- Language: C++
- License: MPL-2.0
- What it does: high-density color icon matrix barcode + decoder pipeline
- Can we reuse code?: Potentially via FFI with MPL-2.0 obligations; not used in MVP
- Ideas to borrow:
  - high-speed visual symbol packing concepts
  - calibration/error-correction strategy
- Risks/limitations:
  - FFI complexity
  - platform packaging complexity
  - license boundary management (MPL-2.0 file-level copyleft)
- Rust-only conflict?: **Partial** (logic should stay Rust; C++ dependency only if unavoidable later)

Related project:

- Project: cimbar
- Repo: https://github.com/sz3/cimbar
- Language: Python
- License: MIT
- Note: useful as reference, but not included in Rust MVP runtime.

## 4) CameraFileCopy / CFC

- Project: CFC
- Repo: https://github.com/sz3/cfc
- Language: C++ (Android project)
- License: MIT
- What it does: Android receiver for cimbar streams
- Can we reuse code?: No direct code use in Rust desktop MVP
- Ideas to borrow:
  - receiver UX and progress/error reporting
  - camera scanning interaction patterns
- Risks/limitations:
  - mobile-specific assumptions
  - tied to cimbar stack
- Rust-only conflict?: **Yes** for direct reuse in MVP; inspiration only

## 5) Airgapped QR Code Transfer

- Project: airgapped-qr-code-transfer
- Repo: https://github.com/mohankumarelec/airgapped-qr-code-transfer
- Language: HTML/JavaScript (Vue + browser scanner/generator)
- License: MIT
- What it does: browser-based chunked file transfer via QR animation
- Can we reuse code?: Allowed by license, but **intentionally not reused** due Rust-only requirement
- Ideas to borrow:
  - sender/receiver workflow clarity
  - chunk progress feedback
  - compression toggle UX
- Risks/limitations:
  - web-camera/browser API assumptions
  - architecture not aligned with Rust-native desktop target
- Rust-only conflict?: **Yes** for direct implementation reuse; no conflict for product inspiration

## Summary decisions

- We will **not copy code** from non-Rust projects for MVP.
- We can reuse architecture/protocol ideas after clean Rust reimplementation.
- ShadowCat currently has no detected open-source license; treat it as design inspiration only.
