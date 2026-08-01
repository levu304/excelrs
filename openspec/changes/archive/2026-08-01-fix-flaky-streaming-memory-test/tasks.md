# Tasks: Fix flaky streaming memory test (6.3)

TDD-flavored: assert the gate stops flaky (unexposed) AND holds deterministically
(exposed), with the 50-sheet round-trip readback unchanged.

## 1. Stabilize the 6.3 heap assertion

- [x] 1.1 Add a guarded-GC helper (`const gc = () => { if (global.gc) { global.gc(); global.gc(); } }`); call before `before` and before `after`; run `expect(delta).toBeLessThan(50MB)` only when `global.gc` is defined, else `console.warn('[6.3] heap assertion skipped: --expose-gc not enabled')` and continue. (`__test__/streaming-bridge.test.ts` ~306-320)
- [x] 1.2 Rename test 6.3 title + comment header `"… constant-memory …"` → `"… bounded output-phase heap growth (input buffered)"`; cite `docs/adr/005-streaming-write-buffering.md`.

## 2. Enable deterministic GC in CI

- [x] 2.1 `package.json`: `"test": "vitest run"` → `"test": "NODE_OPTIONS=--expose-gc vitest run"`.

## 3. Verify (both environments)

- [x] 3.1 `NODE_OPTIONS=--expose-gc npm test`: 6.3 hard `<50 MB` holds, all 11 streaming-bridge tests green.
- [x] 3.2 `npm test` (gc absent): 6.3 soft-skips with warn, 11/11 green, 50-sheet readback round-trip still asserts `toHaveLength(50)`.
