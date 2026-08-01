# Proposal: Fix flaky streaming memory test (6.3)

## Why

`__test__/streaming-bridge.test.ts` test "6.3" asserts `finalizeToFile`'s
`process.memoryUsage().heapUsed` delta stays under 50 MB for a 50-sheet × 1000-row
workbook, measured with neither `global.gc()` nor `--expose-gc` in the test run.
`heapUsed` is nondeterministic under V8 arena growth + CI timing, so the gate
**fails green builds intermittently** — a reliability defect, not a behavior
regression. The test title also says "constant-memory," which contradicts the
confirmed write path (input sheets are buffered in the `StreamWriter` handle;
only *output* is streamed — see `docs/adr/005-streaming-write-buffering.md`).

## What Changes

- Stabilize the 6.3 heap assertion: force GC around the before/after snapshots
  when `global.gc` is available, keep the hard `<50 MB` threshold only on that
  deterministic path, and soft-skip (warn, never red) when `--expose-gc` is absent
  so CI stops red-green-flip-floping.
- Correct the test's premise from "constant-memory" to "bounded output-phase heap
  growth" so it matches actual behavior and ADR-005.
- Enable `--expose-gc` in the `package.json` test script (via `NODE_OPTIONS`) so
  CI measures deterministically; the guard still degrades gracefully elsewhere.

## Capabilities

- **New Capabilities:** *none.*
- **Modified Capabilities:** *none — test/tooling-only change; no spec-level
  behavior change. `skip_specs: true` (see `.openspec.yaml`).*

## Impact

- `__test__/streaming-bridge.test.ts` — rename + guarded assertion (test 6.3 only).
- `package.json` — `test` script gains `NODE_OPTIONS=--expose-gc` (deterministic CI).
- No Rust / `native` / public API changes. `cargo check` and `vitest` (11/11
  streaming-bridge) stay green.
