## Why

`src/stream.rs` carries a verbatim-duplicated `///` doc-comment block directly above `pub fn stream_write_to_file` (current lines 1146-1157: the 6-line comment appears twice consecutively as 1146-1151 and 1152-1157). It is leftover doc debt from the constant-memory streaming-write line of work (the #34 revert / PR #49 bridge). Harmless to `rustc`/`rustdoc` (they neither warn nor error on it) but misleading: a reader scanning for `stream_write_to_file` sees two identical prose blocks. Cleaning it now keeps the upcoming bridge PR free of obvious duplication.

## What Changes

Delete the second copy of the duplicated doc-comment block (current lines 1152-1157) so a single block (1146-1151) documents `stream_write_to_file`. **No code, no API, no behavior.**

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

(none — docs only; `skip_specs: true` set on this change)

## Impact

- `src/stream.rs` only (comment removal).
- `cargo doc` output regains a single canonical comment for `stream_write_to_file`.
- No runtime, no public Rust API, no JS bridge impact.
- **Out of scope observed:** a parallel duplicate doc block above `stream_write_to_memory` was also found by a freq scan. Left untouched here; recommended as a follow-up change to keep this one surgical.
