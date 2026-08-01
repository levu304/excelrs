## Context

(exists already in `index.d.ts`, `src/stream_handle.rs`, `src/stream-bridge.ts`,
`docs/adr/005-streaming-write-buffering.md`, and the deferred
`openspec/specs/streaming-write-incremental/spec.md` as "Deferred (not yet
implemented)"). The shipped `StreamWriter` is a two-phase writer: `write_sheet()`
buffers every sheet into `self.sheets: Vec<StreamSheet>` (O(all sheets)); only
the `finalize*` output phase streams (cap-16 bounded mpsc channel, one sheet's
XML + shared-strings/style accumulators at a time). The three prose specs above
still promise constant-memory *write* I/O as a whole. Only the prose is stale;
no source change is needed or in scope.

## Goals / Non-Goals

**Goals:**

- Align the three writer specs to the shipped two-phase memory model.
- Remove the consumer footgun where `streaming-write-to-file/spec.md`'s
  "at no point shall the process buffer more than one sheet's XML" misleads a
  caller piping a multi-GB workbook through `writeToWritable`.

**Non-Goals:**

- Implement true incremental `writeSheet()` (the `#34` revert class — FFI
  threading + `spawn_blocking` "no reactor running" panic + workbook.xml
  chicken-and-egg on sheet count). Deliberately out of scope; tracked by
  `streaming-write-incremental`.

## Decisions

1. **De-scope is permanent until a scoped spike.** The two-phase model
   (input buffered O(all sheets); output streamed constant-memory) is
   intentional and documented, not a bug to fix here. Wording must match it.
2. **Wording fix only** — edit the requirement bodies and the two
   memory-claim scenarios in `streaming-xlsx` / `streaming-write-to-file` /
   `streaming-write-to-readable` to qualify "constant-memory" to the *output*
   phase and to cross-reference `streaming-write-incremental` + ADR-005.
3. **Preserve true claims.** The read-side constant-memory behavior and
   output-phase backpressure claims are left intact (they are correct).

## Risks / Trade-Offs

- **[Spec-merge risk]** MODIFIED requirement bodies must match the exact
  `### Requirement:` header at archive time to avoid duplicate requirements.
  → Mitigation: `openspec validate` + diff before archive; verify headers
  byte-for-byte against `openspec/specs/*/spec.md`.
- **[Weaker headline]** "constant-memory streaming write" downgrades to
  "constant-memory *output*" on the write path. → Acceptable; honest > marketing,
  and the read path remains genuinely constant-memory end-to-end.

## Open Questions

- Q: Should the capstone `streaming-xlsx` Purpose ("TBD — update after archive")
  be filled in by this change? → No. Stay tight; housekeeping is a separate task.
- Q: When to re-attempt true incremental `writeSheet`? → Gated on a separate
  spike (issue #25.1, #34 revert). Left in the `streaming-write-incremental`
  backlog, not this change.
