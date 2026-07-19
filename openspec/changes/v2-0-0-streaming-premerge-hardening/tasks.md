## 1. A4 — reset `in_f` at cell boundary

- [x] 1.1 In `parse_sheet_rows` (`src/stream.rs`), set `in_f = false` when handling a `</c>` (cell end) event.
- [x] 1.2 Build (`cargo build --lib`) and run `cargo test --lib stream` — confirm no behavior change on existing well-formed tests.

## 2. Multi-sheet pairing round-trip test

- [x] 2.1 Add `stream_read_preserves_multi_sheet_order` test in `src/stream.rs`: write 2–3 sheets via `stream_write` (distinct per-sheet numeric values so each sheet is individually identifiable), then read back via `stream_read`.
- [x] 2.2 Assert returned `StreamSheet` count, document-order names, and per-sheet cell values match what was written.
- [x] 2.3 Run `cargo test --lib stream` — the new test is green.

## 3. Verify & land

- [ ] 3.1 `cargo test --lib stream` fully green (existing + new test).
- [ ] 3.2 Confirm `index.d.ts` / `package.json` unchanged.
- [ ] 3.3 Commit on `v2-0-0-streaming-parity-capstone` branch; push; confirm CI green on PR #24.
