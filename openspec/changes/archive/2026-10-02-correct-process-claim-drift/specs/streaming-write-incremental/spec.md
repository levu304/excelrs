# Spec Delta

## MODIFIED Requirements

### Requirement: The writeToWritable bridge releases the stream on early exit

The JS consumer bridge (`writeToWritable`) MUST release or abandon the stream on its own
early exit, performing `readable.cancel()` and `reader.releaseLock()` in a `finally`. This
hardens output-phase teardown only; it does NOT implement true incremental `writeSheet()`
(the streaming write buffering decision, out of scope).

#### Scenario: Bridge tears down on early exit

- **WHEN** `writeToWritable` exits before the stream is fully drained
- **THEN** it SHALL run `readable.cancel()` and `reader.releaseLock()` in a `finally` block

#### Scenario: Bridge teardown does not implement incremental writeSheet

- **WHEN** the bridge change is reviewed against the streaming write buffering decision
- **THEN** it SHALL be limited to output-phase teardown and SHALL NOT introduce true incremental `writeSheet()`