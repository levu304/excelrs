# Design: Fix flaky streaming memory test (6.3)

## Context

PR #49 shipped the streaming writer. Per `docs/adr/005-streaming-write-buffering.md`,
output is streamed (bounded mpsc channel, cap 16) while **input sheets are
buffered in the `StreamWriter` handle** before `finalize`. Test 6.3
(`__test__/streaming-bridge.test.ts:288`) measures `process.memoryUsage().heapUsed`
around `finalizeToFile` only — i.e. the **output phase**, after the
`writeSheet` loop has already buffered all 50 sheets. It asserts `delta < 50 MB`.
CI runs `vitest run` with **no `--expose-gc`**, so no GC is forced between the
`before`/`after` snapshots; `heapUsed` reflects V8 arena growth + fragmentation,
which is timing-dependent and red-green-flip-flops on green builds.

## Goals / Non-Goals

**Goals:**

- Eliminate red-on-green from test 6.3.
- Guard the *output-phase* heap invariant deterministically (the claim worth
  keeping given output is the streamed part).
- Align the test's narrative with ADR-005: input is buffered; **output** is
  streamed/bounded.

**Non-Goals:**

- Change the 50 MB threshold semantics or the 50-sheet × 1000-row workload.
- Any Rust / `native` / public-API change.
- True constant-memory `writeSheet()` (Path A — out of scope; tracked in
  ADR-005 / `openspec/specs/streaming-write-incremental`).

## Decisions

- **D1 — Force GC on the deterministic path.** When `global.gc` is available, call
  it (twice, for V8 generational) immediately before `before` and immediately
  before `after`. This is the only way `heapUsed` delta reflects `finalizeToFile`'s
  own allocations rather than uncollected prior garbage.
- **D2 — Soft-skip when GC is unavailable.** If `--expose-gc` is absent
  (`global.gc` undefined), do **not** run the numeric assertion — `console.warn`
  a clear skip message instead and let the 50-sheet round-trip readback still
  assert. Rationale: a nondeterministic threshold that flips green→red is worse
  than no gate; an honest skip is preferable to a flaky "constant-memory" check.
- D3 — Enable `--expose-gc` in the `package.json` script via `NODE_OPTIONS` so CI
  takes the deterministic path. Inline `NODE_OPTIONS=--expose-gc vitest run`
  adds no dependency. D2 keeps non-CI / Windows-local environments green if the
  flag doesn't propagate.
  - *Alt considered*: vitest `nodeOptions` config. Rejected — adds config
    surface; `NODE_OPTIONS` is minimal and already understood.
  - *Alt considered*: drop the memory assertion entirely. Rejected — loses the
    regression signal ADR-005 makes worth keeping.
- **D4 — Rename the narrative.** Rename test 6.3 + its comment header
  `"constant-memory"` → `"bounded output-phase heap growth (input buffered)"`,
  citing ADR-005. Keeps the test's intent honest.

## Risks / Trade-offs

| Risk | Likelihood | Mitigation |
| --- | --- | --- |
| `NODE_OPTIONS=--expose-gc` not portable to Windows CI | Low | Repo CI is Linux; D2 guard degrades gracefully elsewhere. Re-add `crossenv` if Windows CI introduced. |
| CI flag doesn't reach the vitest worker → memory gate silently skipped | Medium | D2 logs an explicit `[6.3] heap assertion skipped` warning; flag monitored in CI logs. Revisit if coverage drops. |
| 50 MB threshold still empirical | Low | Comment states the workload explicitly; retune with evidence on drift. |
| `global.gc` double-call cost in CI | Negligible | GC of a 50-sheet workbook is sub-millisecond; not on hot path. |

## Migration Plan

- Test-only; no production cut.
- After change, run `npm test` **with and without** `--expose-gc`:
  - exposed: `<50 MB` hard assert holds deterministically.
  - unexposed: assertion soft-skips; 50-sheet round-trip readback still passes.
- No rollback needed; revert touches only the test file + `package.json` script.

## Open Questions

- *None blocking this change.* Whether a true constant-memory write path (Path A)
  is ever a product requirement is tracked in ADR-005 and `streaming-write-incremental`;
  it does not gate stabilizing test 6.3.
