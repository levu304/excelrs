## ADDED Requirements

### Requirement: finalizeToReadable is cancelable + self-cleaning

`finalizeToReadable`'s detached zip-writer worker MUST terminate promptly (≤2 s,
not the ~55–60s GC window) and release the `ZipWriter`, `StreamSession`, and
bounded mpsc channel whenever the consumer abandons the `ReadableStream`,
whether by **explicit cancel** or by **drop-without-release**.

A *live* consumer that keeps draining MUST receive all chunks exactly once in
order, with cap-16 backpressure preserved (byte-identical emission to prior
behavior); cancellation/abandon MUST NOT surface as a write error to a live
consumer and MUST NOT corrupt the zip for a live reader.

The JS consumer bridge (`writeToWritable`) MUST release/abandon the stream on its
own early exit (`readable.cancel()` + `reader.releaseLock()` in a `finally`) —
this change does NOT implement true incremental `writeSheet` (ADR-005 Path A,
out of scope); it hardens output-phase teardown only.

#### Scenario: Explicit cancel terminates the worker promptly

- **WHEN** `finalizeToReadable` is mid-emit and the consumer calls
  `readable.cancel()` before the zip writer finishes
- **THEN** the detached worker thread exits within a bounded window (≤2 s),
  the held `ZipWriter` + `StreamSession` are dropped, and the channel reports
  EOF/`Closed` (no ~55–60s GC wait)

#### Scenario: Live consumer still gets full output plus backpressure

- **WHEN** a consumer reads the full `ReadableStream` normally (does not cancel)
- **THEN** all zip chunks are delivered exactly once in order and consumer-driven
  backpressure is preserved (cap-16 respected, no spin, live path byte-identical
  to prior behavior)

#### Scenario: Abandon-without-cancel still terminates (Rust backstop)

- **WHEN** a caller drops/cancels at the JS level but napi defers dropping the
  underlying Rust `Stream` (B2 uncertainty)
- **THEN** the Rust worker MUST still self-terminate (does not park forever),
  because `ChannelWriter::write` no longer relies solely on
  `blocking_send`/receiver-drop — it checks an `is_closed()`/`AtomicBool` guard
  and uses non-parking `try_send`
