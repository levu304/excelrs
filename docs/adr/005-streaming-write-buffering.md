# ADR-005 — Streaming writer: input is buffered, not constant-memory

- **Status:** Accepted (Path B)
- **Date:** 2026-08-02
- **Scope:** `src/stream_handle.rs` `StreamWriter`, `src/stream-bridge.ts`, `index.d.ts`
- **Deciders:** lev

## Context

The streaming XLSX writer (`StreamWriter`) promises "constant-memory" on
`finalizeToFile` / `finalizeToReadable` / `writeToWritable` and ships the
`openspec/specs/streaming-write-incremental` spec stating it *"SHALL write
each sheet's XML directly to the zip writer as sheet provided"* — yet the
implementation is:

```rust
pub fn write_sheet(&mut self, sheet: JsStreamSheet) -> Result<()> {
    self.sheets.push(from_js_sheet(&sheet));   // accumulates ALL sheets
    Ok(())
}
```

Every `writeSheet()` call buffers the whole workbook's sheets into
`self.sheets: Vec<StreamSheet>`; each `finalize*` then clones that `Vec` and
hands it to `StreamSession::write_sheet_xml` in a loop. Consequences:

- **Peak write memory is O(all sheets)**, not O(one sheet). The "constant-memory"
  claims are true only for the *output-emission* stage (bounded mpsc channel,
  cap 16 backpressure), which is the part that actually streams.
- A spec (`streaming-write-incremental`, authored + archived by PR #49) is now
  part of the committed project contract while the code violates it.

### Prior attempt, unrecorded revert

A previous attempt at this same bridge was PR #34
(`f37cc2c`, "constant-memory streaming XLSX bridge", issue #25.1). It was
reverted on 2026-07-20 by commit `c19a4fc`, which landed **directly on `main`**
with a bare message — *"This reverts commit f37cc2c…"* — and **no recorded
rationale** (no PR, no issue comment, no ADR). Its `code-review-pr` run flagged
(among others):

- 🔴 `writeToWritable` never called `writable.end()`; piped consumers hung
  (output buffered into one `Buffer` before writing any byte).
- 🔴 `StreamReader` opened `ZipArchive` with no entry-count/size cap (DoS).
- 🟡 `next()` re-parsed the archive on every sheet (perf).

Plus a Medium/Low *"write-path buffering claim"* that was intentionally not
posted inline.

PR #49 re-architected the bridge: moved `finalizeToReadable`'s zip work onto a
`std::thread::spawn` (commit `96e877b` *"fix: streaming finalizeToReadable no
longer panics"* — `spawn_blocking` panicked on the JS thread with *"no reactor
running"*) and redid `writeToWritable` to drain a `ReadableStream` into the
`Writable` with `end()`/`finish`/`error` handling. These **fixed the two fatal
defects above**. The read-side DoS cap shipped separately later (#25.3).

**Inferred revert driver for #34 (not recorded anywhere):** a runtime
panic/​hang in the JS-thread bridge (the *"no reactor running"* class that #49
itself had to fix) combined with the never-ends-writable hang — severe enough
to revert and re-architect. This inference is consistent with #49's explicit
panic-fix commit, but is **inference**, not documented fact.

## Decision

**Path B — de-scope + honest claims.** Ship #49 as-is (tested, panic-fixed,
output-streaming) but stop overselling:

1. Rewrite the "constant-memory" claims on `finalizeToFile`, `finalizeToReadable`,
   and `writeToWritable` (`src/stream_handle.rs`, `index.d.ts`, `src/stream-bridge.ts`)
   to state precisely: **output** is streamed with backpressure; **input**
   sheets are buffered in the handle (peak memory O(all sheets)).
2. Mark `openspec/specs/streaming-write-incremental/spec.md` **Deferred / target
   for a future change** (not deleted — it remains the roadmap for true
   incremental `writeSheet`).
3. Record this reasoning here so the #34 revert lesson and the buffering
   tradeoff are not re-learned.

### Non-decisions (explicitly deferred)

- **Implement true incremental `writeSheet`.** That requires the handle to own
  an open `ZipWriter<File>` (or `new_stream` writer) across multiple
  `#[napi]` `writeSheet` calls, defer `sharedStrings`/`styles`/central-directory
  - `finish()` until `finalize`, and surface mid-stream write errors through the
  FFI boundary — the exact shape that got PR #34 reverted. Tracked separately
  as the `streaming-write-incremental` follow-up. Reattempting it now, during a
  panic-fix review, is out of scope.

- **Re-attempt the async `spawn_blocking` bridge.** The panic on the JS thread
  (`spawn_blocking` needs a tokio reactor; sync `#[napi]` fns have none) is the
  reason #49 uses `std::thread::spawn`. Switching back re-introduces the
  #34-class failure.

## Consequences

- **Positive:** Claims match behavior. Users who chain `StreamReader`→`writeToWritable`
  get real backpressured *output* streaming; they are no longer told they have
  constant-memory when a 50-sheet-MB workbook still loads all sheet data into the
  handle. No behavior change; tests unchanged.
- **Negative:** The headline "constant-memory streaming write" is downgraded to
  "output-streaming with backpressure." Users needing true O(one-sheet) write
  memory still lack a path — but they lacked it before #49 too (the promise was
  false). The unimplemented spec remains in-tree as a deferred target rather
  than a shipped violation.
- **Risk if Path A (true incremental) wanted later:** re-implementing
  `writeSheet` across FFI requires re-surviving the #34 failure mode — do it as a
  dedicated, well-scoped change with its own spike, not folded into this.

## References

- Issue #25.1 (deferred follow-up from PR #24 v2.0.0).
- PR #34 (`f37cc2c`) and its revert `c19a4fc` (no recorded rationale).
- PR #49 `96e877b` "fix: streaming finalizeToReadable no longer panics".
- `src/stream_handle.rs` `finalize_to_readable` / `finalize_to_file` (panic fix).
- `openspec/specs/streaming-write-incremental/spec.md` (this spec's target).
