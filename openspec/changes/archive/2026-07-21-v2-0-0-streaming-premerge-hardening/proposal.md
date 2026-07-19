## Why

PR #24 (change `v2-0-0-streaming-parity-capstone`) ships the v2.0.0 streaming XLSX reader/writer. A qa-expert audit of the three fix commits (issue #26) confirmed two residual risks that are cheap, behavior-preserving, and safe to land *before* the v2.0.0 release cut: a malformed-XML formula-flag leak (A4) and the absence of a multi-sheet pairing regression test. Both harden the release without behavioral risk.

## What Changes

- **A4 — reset the `in_f` formula-capture flag at cell boundary.** In `parse_sheet_rows` (`src/stream.rs`), `in_f` is set `true` on `<f>` Start and cleared on `</f>` End. If `</f>` never arrives (malformed/truncated XML), the flag stays set and the *next* cell's text leaks into the previous cell's formula buffer. Fix: clear `in_f = false` when a `</c>` (cell end) is handled. One-line defensive guard; no behavior change on well-formed XML.
- **Add a multi-sheet pairing round-trip test.** A new Rust test (`stream_read_preserves_multi_sheet_order`) builds an in-memory multi-sheet `.xlsx`, runs `stream_read`, and asserts correct sheet count, document-order names, and per-sheet cell values. This is a regression net for the sheet name↔file pairing/order path — the exact code A1 corrupts — without changing any behavior now.

## Capabilities

### New Capabilities

<!-- none: this is a hardening/bug-fix change with no new public capability -->

### Modified Capabilities

- `streaming-xlsx`: adds a malformed-input robustness guarantee — the reader SHALL reset formula-capture state at each cell boundary so a missing `</f>` cannot leak the next cell's value into the prior cell's formula (audit risk A4). No public API change.

## Impact

- **Code:** `src/stream.rs` only (`parse_sheet_rows` + test module). No `index.d.ts`, `src/stream_handle.rs`, or `package.json` changes.
- **Branch:** implemented on `v2-0-0-streaming-parity-capstone` (the PR #24 branch), committed before the v2.0.0 release tag.
- **Out of scope (tracked in #26, post-merge):** A1 (digit-greedy filename parse → wrong file pair — a *behavioral* change, deferred for a cleaner rels-target-keyed fix) and A3 (declared-size zip-bomb cap — library-wide input-trust gap; the existing `Workbook.xlsx.read` has *no* cap at all).
