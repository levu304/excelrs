## Context

`finalize_to_readable` (`src/stream_handle.rs:458`) drives the zip writer on a
detached `std::thread::spawn` (sync `#[napi]` fn: no tokio runtime, so
`spawn_blocking` panics — the #34/#49 failure class ADR-005 warns about. **Not
attempting that rewrite here.**). Grounded facts (src/stream.rs):

- `ReadableStreamSession = StreamSession<zip::write::StreamWriter<ChannelWriter>>` (line 231).
- `write_sheet_xml(sheet)` writes through `self.zip` ⇒ `ChannelWriter::write` ⇒
  `sender.blocking_send(Ok(chunk))` (line 273) — park site during **sheet writes**,
  not only finalize.
- `finalize_to_channel` (line 212) → `self.zip.finish()` flushes all buffered bytes
  (central dir + every part) through `ChannelWriter::write` — a burst of
  `blocking_send`; if consumer abandoned, parks for the full leak window.
- cap-16 channel (`tokio::sync::mpsc::channel(16)`, line 243).
- EOF relies on the caller's sender clone dropping after the session
  (comment lines 206–211) — unreachable while the worker is parked.
- Worker closure also does `sender.blocking_send(Err(e))` on write error
  (`stream_handle.rs:470`) — same park risk on the error path.

The **actual** abandon path lives in JS: `writeToWritable`
(`src/stream-bridge.ts:97`) rejects/resolves on writable `error`/`close`/`finish`
**without** `reader.releaseLock()` / `readable.cancel()` (ADR-005 §Scope lists
`src/stream-bridge.ts` — this touch is in-surface, not a new scope).

Tokio resolution (B1): `tokio::sync::mpsc` docs + Oxide RFD #609 — dropping the
last `Receiver` closes the channel and wakes a sender parked in `blocking_send`
⇒ `Err(Closed)`. So IF the Rust `ReceiverStream` is dropped, the worker exits.
The only empirical residual (B2) is whether napi drops the Rust `Stream` promptly
on JS `ReadableStream.cancel()` or only on GC — the Rust layer below makes that
**irrelevant to correctness**.

## Goals / Non-Goals

**Goals:**

- Worker thread terminates within ≤2 s on consumer cancel OR abandon (close the
  ~55–60s GC-window leak); release zip writer + session + channel.
- Live-consumer output byte-identical (cap-16 backpressure, same chunk order).

**Non-Goals:**

- True incremental `writeSheet` (ADR-005 Path A — defer; this is teardown-only).
- Async `spawn_blocking` bridge rewrite (ADR-005 "re-attempt… out of scope" —
  sync `#[napi]` fn has no runtime).
- Exposing a new JS cancel() API beyond std `ReadableStream.cancel()`/`releaseLock()`.

## Decisions

- **Two layers; JS layer is the prompt path, Rust layer is the correctness floor.**
  JS `writeToWritable` calls `reader.releaseLock(); readable.cancel()` in a
  `finally` on every exit (cancel needs an unlocked stream per the Web Streams
  spec; releaseLock() is best-effort, guarded) — this is the ~55–60s→~0s lever (nAPI drops the Rust
  stream ⇒ B1 fires). Rust layer (`try_send` + `is_closed()`/`AtomicBool`)
  guarantees termination *even if* napi defers the Rust drop (B2) or for raw
  `finalizeToReadable` callers that never cancel — removes the only park-forever
  site regardless of B2. The proposal's prior "zero JS-side change" was wrong.
- **`try_send` + `is_closed()` instead of `blocking_send` in `ChannelWriter::write`**
  (the single choke point for both sheet-write and `finish()` flush): `Full` +
  cancelled / `Closed` ⇒ `Err` (unwinds zip; worker exits via early-return +
  session drop); `Full` + alive ⇒ `sleep` backoff (cap-16 backpressure preserved;
  worker is a std thread with no runtime, so a runtime-native park would re-panic).
- **`Drop for ReceiverStream` sets a shared `Arc<AtomicBool>`** — explicit intent
  - defense-in-depth; no reliance on tokio wake timing for the flag.
- **Same `try_send` fix on the terminal `sender.blocking_send(Err(e))`** path
  (`stream_handle.rs:470`) — it parks on a full channel too.
- **`AtomicBool::Relaxed`** — single producer/consumer, only flag visibility before
  next `try_send`; sufficient. No lock around sender.

## Risks / Trade-offs

- Sleep-yield backoff less CPU-tight than park on the live slow-consumer path
  (cap-16 bursts during `finish()`). `ponytail: bounded by cap16 + 5ms yield;
  upgrade = crossbeam park/unpark signaled by`ReceiverStream::drop`for true
  instant backpressure — deferred.`
- **B2 unresolved empirically** (napi cancel→Rust-drop timing) — mitigated by the
  Rust floor layer: correctness does not depend on it; B2 only affects *promptness
  in the JS-cancel path*, which the JS bridge `finally.cancel()` already makes
  prompt. Tracked as task 3.3 validation, not a design risk.
- `write` returning `Err` mid-zip leaves writer half-finished; session dropped,
  no partial file exposed to a *live* consumer (they never take the cancel path).
- **Regression-guard against ADR-005 #34 lesson:** this is NOT an async-bridge or
  incremental-writeSheet attempt — it adds lifecycle cleanup to the existing
  sync detached-thread design; no FFI-shape rewrite.

## Open Questions

- Confirm B2: does `create_with_stream_bytes` drop the Rust `Stream` on JS
  `ReadableStream.cancel()` (vs GC)? Validated by task 3.3 / a cancellation
  repro — determines only promptness in the JS-cancel path, not correctness.
- Backoff granularity 5 ms vs 10 ms — tuned from spike.
- Is `tokio::sync::mpsc::Sender::is_closed()` + `try_send` stable in the
  pinned tokio (1.x `sync`)? (Assume yes; verify in build.)
