## REMOVED Requirements

### Requirement: finalizeToReadable is cancelable + self-cleaning

**Reason**: Combined the worker-termination guarantee, the live-consumer delivery guarantee, and the JS bridge teardown rule into one over-long requirement.

**Migration**: Replaced by three requirements: worker termination, live-consumer delivery, and bridge teardown. The capability's status as the target for deferred true incremental `writeSheet()` is unchanged.

## ADDED Requirements

### Requirement: finalizeToReadable releases its worker and resources on consumer abandon

`finalizeToReadable`'s detached zip-writer worker MUST terminate promptly (≤2 s, not the
~55–60s GC window) and release the `ZipWriter`, `StreamSession`, and bounded mpsc channel
whenever the consumer abandons the `ReadableStream`, whether by **explicit cancel** or by
**drop-without-release**.

#### Scenario: Explicit cancel releases the worker

- **WHEN** a consumer calls `readable.cancel()` mid-stream
- **THEN** the detached worker SHALL terminate promptly and release its zip writer, session, and channel

#### Scenario: Drop-without-release releases the worker

- **WHEN** a consumer drops the `ReadableStream` without releasing it
- **THEN** the detached worker SHALL still terminate promptly and release its resources

#### Scenario: Worker exits within a bounded window

- **WHEN** the consumer abandons the stream and the detached worker is joined
- **THEN** the worker SHALL exit within ≤2 s, not after a ~55–60s GC wait

### Requirement: A live finalizeToReadable consumer receives all chunks exactly once

A *live* consumer that keeps draining MUST receive all chunks exactly once in order, with
cap-16 backpressure preserved. Cancellation or abandonment MUST NOT surface as a write error
to a live consumer and MUST NOT corrupt the zip for a live reader.

#### Scenario: Live consumer gets every chunk in order

- **WHEN** a consumer drains the `ReadableStream` to completion
- **THEN** it SHALL receive all chunks exactly once in order, byte-identical to prior behavior, with cap-16 backpressure preserved

#### Scenario: Abandonment does not corrupt a live reader

- **WHEN** one stream is abandoned while a separate live reader consumes the same zip
- **THEN** the live reader SHALL observe no write error and no zip corruption

### Requirement: The writeToWritable bridge releases the stream on early exit

The JS consumer bridge (`writeToWritable`) MUST release or abandon the stream on its own
early exit, performing `readable.cancel()` and `reader.releaseLock()` in a `finally`. This
hardens output-phase teardown only; it does NOT implement true incremental `writeSheet()`
(ADR-005 Path A, out of scope).

#### Scenario: Bridge tears down on early exit

- **WHEN** `writeToWritable` exits before the stream is fully drained
- **THEN** it SHALL run `readable.cancel()` and `reader.releaseLock()` in a `finally` block

#### Scenario: Bridge teardown does not implement incremental writeSheet

- **WHEN** the bridge change is reviewed against ADR-005
- **THEN** it SHALL be limited to output-phase teardown and SHALL NOT introduce true incremental `writeSheet()`