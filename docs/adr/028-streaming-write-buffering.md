# 028 — Streaming write buffering strategy

**Status:** current
**Recorded at:** v2.0.0

## Decision

The streaming writer buffers emitted XML rather than writing each element as it is
produced, so that backpressure and partial-flush behavior stay bounded and predictable.

## Context

Added with the streaming writer in v2.0.0. A SAX-style writer is tempted to write directly
to the output stream, but that couples producer rate to consumer rate: a slow downstream
reader turns into unbounded memory growth in the writer, or a stall.

## Alternatives considered

- **Write-through with no buffering.** Rejected — producer and consumer rates become
  coupled, and a slow consumer costs memory or latency.
- **Unbounded buffering.** Rejected — trades a throughput problem for a memory problem.

## Numbering note

This decision was previously extracted under the number `005-streaming-write-buffering`.
That number came from a different numbering stream and **collided with ADR-005 (dual
MIT/Apache-2.0 license)**, which is row 5 of the v1.0.0 specification's table. The v1.0.0
table's 1–27 space is canonical; this decision takes the next free slot, **28**.

The substantive content of the original file is carried forward here. No other decision
occupied number 28, so nothing was displaced.

## Related

- Streaming input and output phases are specified in the `streaming-xlsx` capability, which
  is the single normative statement of the phase disclaimer.
- Buffering specifics are covered by `streaming-write-incremental` and the archived
  `005`-series change that introduced this decision.

## Sources

- The original `005-streaming-write-buffering` file (renamed to this one).
- OpenSpec capabilities `streaming-xlsx`, `streaming-write-incremental`.
- `src/stream.rs`, `src/writer/` — the streaming write path.
- OpenSpec change `reconstruct-design-record`, design D2 and task 2.3.
