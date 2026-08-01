## Why

`finalize_to_readable` (`src/stream_handle.rs:458`) spawns a detached
`std::thread::spawn` that drives a `zip::ZipWriter<ChannelWriter>` and pushes
chunks through `ChannelWriter::write` → `tokio::sync::mpsc::Sender::blocking_send`
into a cap-16 channel drained by the JS `ReadableStream`.

If the JS consumer abandons that stream before the writer finishes, the channel
fills to 16 and `blocking_send` **self-parks the worker** — because the worker is
a detached `std` thread with no tokio runtime (`spawn_blocking` panics here:
"no reactor running"), nothing unparks it. It only exits when the Rust
`ReceiverStream` is dropped (channel closes ⇒ tokio wakes the parked sender ⇒
`Err(Closed)`). That Rust drop is gated by **JS GC of the abandoned stream**, so
the *observed* leak is the GC interval (~55–60s under the `--expose-gc` fixtures
in `__test__/streaming-bridge.test.ts:6.3`), not hard process teardown.

Two root facts:

- **B1 (tokio, resolved by docs):** receiver-drop closes the channel and wakes a
  parked `blocking_send` ⇒ `Err(Closed)`. So IF the Rust stream is dropped, the
  worker exits.
- **B2 (napi, empirical):** does `create_with_stream_bytes` drop the Rust source
  `Stream` promptly on JS `ReadableStream.cancel()`, or only on GC?

There is **no JS→Rust cancel path today**: `writeToWritable` (`stream-bridge.ts`)
rejects/resolves without calling `reader.releaseLock()` / `readable.cancel()`,
so the real-world abandon path relies entirely on GC — that is the leak.

## Changes

Two layers; both required (independent of B2):

- **JS consumer-bridge** (`src/stream-bridge.ts` `writeToReadable`'s `writeToWritable`,
  explicitly in ADR-005 §Scope) — on early exit (`reject`/`resolve`/`error`)
  call `reader.releaseLock(); readable.cancel()` in a `finally` (cancel needs an unlocked stream per the Web Streams spec; releaseLock() is best-effort, guarded). This is the
  **~55–60s → ~0s lever**: it makes napi drop the Rust stream promptly so B1 fires
  without waiting for GC. Pure lifecycle call; not an async-bridge or
  incremental-`writeSheet` rewrite (ADR-005 §Non-decisions: true incremental
  `writeSheet` is a *separate, deferred* change — this touch neither).
- **Rust worker hardening** (`src/stream.rs` `ChannelWriter::write` L273;
  `src/stream_handle.rs:458` worker) —
  - in `ChannelWriter::write`, replace `blocking_send` with `try_send` + a
    `Full`-backoff (5 ms sleep) and a `Closed`-arm returning `Err`: `Full`+alive
    ⇒ yield (cap-16 backpressure preserved); `Closed` ⇒ `Err` (consumer gone). No
    `Arc<AtomicBool>`: tokio `Sender::is_closed()` + the `Closed` variant of
    `try_send` are authoritative, so `ReceiverStream<T>` stays unchanged (no struct
    /signature changes, no `impl Drop` needed).
  - same `try_send` swap for the terminal `sender.blocking_send(Err(e))` path
    (`stream_handle.rs:470`);
  This removes the **only park-forever site** for *any* caller — including raw
  `finalizeToReadable` used without an explicit `cancel()` — so correctness does
  not depend on B2.

**No new dependency.** Live-consumer path is byte-identical (cap 16, same chunk
order, backpressure unchanged).

## Capabilities

### Modified Capabilities

- `streaming-write-incremental` — adds a *cancellation/abort* guarantee to
  `finalizeToReadable` + its JS consumer (`writeToWritable`). Now spans
  `src/stream.rs`, `src/stream_handle.rs` **and** `src/stream-bridge.ts`
  (the bridge is already ADR-005 in-scope). New behavior ⇒ ADDED requirement in
  the delta spec (`specs/streaming-write-incremental/spec.md`). **Status stays
  Deferred** w.r.t. ADR-005 (input buffering `O(all sheets)` unchanged; true
  incremental `writeSheet` is the separate deferred Path-A item, not touched here).
