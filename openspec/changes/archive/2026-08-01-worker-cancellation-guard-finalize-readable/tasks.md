## 1. JS consumer-bridge cleanup (prompt cancel path)

- [x] 1.1 Add `finally { reader.releaseLock(); readable.cancel() }` (best-effort)
  to `writeToWritable` (`src/stream-bridge.ts`) so early exit
  (reject/resolve/error/close) promptly signals the native stream to drop.
  `cancel()` needs an unlocked stream (Web Streams spec; guarded against the
  in-flight-read throw). `reader.releaseLock()` is best-effort via `ignoreErr`.
  ADR-005 §Scope in-surface; not an async-bridge rewrite.
- [x] 1.2 Add vitest `6.4 cancel() on finalizeToReadable settles within 2s` —
  releaseLock → cancel resolves ≤2s, asserts JS-bridge cleanup wires + non-hang.
  Paired w/ 3.3 Rust unit = two-layer guard.

## 2. Rust worker hardening (correctness floor; independent of B2)

- [x] 2.1 Remove the only park-forever site: `ChannelWriter::write` uses `try_send` +
  `tokio` `Closed`/`Full`-backoff (no `Arc<AtomicBool>`; tokio `is_closed`/the
  `Closed` variant are authoritative). `ReceiverStream<T>` unchanged — no
  struct/signature changes (verified `tokio::sync::mpsc::error::TrySendError`).
- [x] 2.2 Replace `blocking_send` in `ChannelWriter::write` (`src/stream.rs`) with
  `try_send` + `Full` 5ms backoff + `Closed`→`Err(BrokenPipe)`: `Full`+alive ⇒
  yield (cap-16 backpressure preserved); `Closed` ⇒ return `Err` (consumer
  gone) ⇒ worker unwinds & exits.
- [x] 2.3 Replace terminal `sender.blocking_send(Err(e))` (`src/stream_handle.rs`,
  ×2 in the drive closure) with `let _ = sender.try_send(Err(e));` (same
  park-forever fix on the error path).

## 3. Verify

- [x] 3.1 `cargo test --lib` — 409 passed; 0 failed (incl. the 11 existing
      streaming tests + new `stream::channel_writer_cancel_tests`).
- [x] 3.2 `cargo clippy --lib -- -D warnings` clean.
- [x] 3.3 Regression guard. B1: `#[cfg(test)]` `channel_writer_cancel_tests::
      write_returns_err_when_receiver_dropped_no_park` — asserts `write()`
      returns `Err` <50ms with no park on receiver-drop (the park-forever guard;
      JS cannot observe native thread count). B2 promptness: validated via 1.2
      JS repro (cancel propagation).

## Notes

- ADR-005 guardrail: teardown/cancel discipline ONLY; NOT incremental `writeSheet`
  (deferred Path A) nor async-bridge rewrite (deferred / #34 revert lesson).
- Expected log on cancel: `ZipWriter::drop: failed to finalize archive:
  Io(Custom { BrokenPipe, "stream consumer gone" })` — the intentional
  worker-unwind path (worker returns on `Closed` → session/zip drop), not a
  failure.
- Build gate: native `.node` rebuilt (`npm run build`) so 1.2/6.4 runs against
  the `try_send` Rust path; `npx tsc --noEmit` clean; biome clean.
