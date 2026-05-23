# StareDrop MVP Roadmap

## Current phase status

- Phase 0: **In progress (mostly complete)**
- Phase 1: **In progress (functional static QR path implemented)**
- Phase 2+: **Not started**

## Phase 0 checklist

- [x] Cargo workspace created
- [x] Modular crate layout created
- [x] `eframe/egui` app skeleton created
- [x] Docs scaffold created
- [x] Existing projects reviewed with license notes
- [x] Core protocol/chunking utility tests added
- [ ] CI config (pending)

## Phase 1 checklist

- [x] Terminal-driven sender mode
- [x] QR encoding (`qrcode`)
- [x] Fullscreen QR display surface
- [x] Terminal-driven receiver mode
- [x] Camera capture loop (`rscam`)
- [x] QR decode loop (`rqrr`)
- [x] Fullscreen camera scan surface
- [x] Decoded text in overlay + terminal output
- [x] Basic logging setup

## Phase 2 preview

Next milestone is small file transfer over animated QR:

1. manifest/data JSON frames
2. chunking + reassembly in app flow
3. repeated frame animation
4. integrity verification and save dialog
