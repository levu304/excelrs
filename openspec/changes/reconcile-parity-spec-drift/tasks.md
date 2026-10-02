# Tasks

Ordering is load-bearing: §6 adds a gate that fails on the very conditions §1–§5 fix, so
it is last and conditional on §1–§5 being green. See `design.md` D2.

Deltas in `specs/` (`exceljs-parity`, `release-verification`, `spec-integrity`) are applied
to the main specs at **archive** time, not here. Tasks that say "edit
`openspec/specs/<cap>/spec.md` directly" are the ones the spec-delta mechanism cannot
express: Purpose sections and the `musl-support` structural repair.

## 1. Restore spec corpus validity (gate-independent)

- [x] 1.1 Repair `openspec/specs/musl-support/spec.md`: replace the `## ADDED Requirements` / `## REMOVED Requirements` / `## MODIFIED Requirements` headers with a single `## Requirements`, drop the `(none)` REMOVED/MODIFIED blocks, and verify all three requirement names and requirement bodies are byte-identical to before the edit (`git diff` shows only header and empty-block lines)
- [x] 1.2 Verify the repair with `openspec list --specs --json` and confirm `musl-support` now reports a requirement count of 3 (was 0)
- [x] 1.3 Verify the repair with `openspec validate musl-support --type spec` and confirm it reports valid with no issues

## 2. Author the placeholder Purpose sections

18 capabilities carry the archive-time `TBD - created by archiving change ...` placeholder.
`design.md` D6 deliberately does not extend to rewriting spec bodies — each Purpose is one
or two sentences of what the capability is for, and nothing else changes in the file.

- [x] 2.1 Write Purpose sections for the workbook-IO group (`auto-filter`, `comments`, `conditional-formatting`, `csv`, `data-validation`) and verify each no longer contains `TBD`
- [x] 2.2 Write Purpose sections for the content group (`date-cell-value`, `dimension-properties`, `headers-footers`, `hyperlinks`, `images`, `merge-range-writer`) and verify each no longer contains `TBD`
- [x] 2.3 Write Purpose sections for the structure group (`page-setup`, `sheet-protection`, `tables`, `workbook-views`, `worksheet-views`) and verify each no longer contains `TBD`
- [x] 2.4 Write Purpose sections for the two large capabilities (`streaming-xlsx`, `release-verification`) and verify each no longer contains `TBD`
- [x] 2.5 Verify the whole set with `openspec validate --specs --strict --json --no-interactive` and confirm the placeholder-Purpose WARNING count drops from 18 to 0

## 3. Reconcile the parity matrix and stale version headers

- [x] 3.1 In `ROADMAP.md`, set the `State (visible/hidden)`, `Tab color`, and `Properties (defaultRowHeight, etc.)` rows to `shipped`, citing the release that carried them, and remove "Not implemented" from their Notes — verify against `worksheet-metadata` (`fd4e84c`) and the `worksheet-metadata` spec that each is genuinely round-trip verified
- [x] 3.2 In `ROADMAP.md`, update the header block (excelrs version, and the "Next: v2.0.0 is the planned capstone" note) to the current released version, and verify no line still describes v2.0.0 as pending
- [x] 3.3 In `docs/spec.md`, correct the `**Version:**` and `**Next:**` header fields so they no longer claim v1.0.0 / "v2.0.0 is the planned capstone", and verify the header no longer contradicts `package.json`'s version
- [x] 3.4 Grep `ROADMAP.md` and `docs/spec.md` for remaining stale version claims (`v1.1.0`, `excelrs version: 1.1.0`, "v2.0.0 is now in implementation") and correct each; leave the v1.x body content of `docs/spec.md` alone per `design.md` D6

## 4. Confirm the deltas are consistent with the reconciled state

The `exceljs-parity` and `release-verification` deltas fix the obsolete `SHALL`s at archive
time. Check now that they will not contradict §3's edits.

- [x] 4.1 Re-read `specs/exceljs-parity/spec.md` in this change and confirm its rewritten "parity program complete" requirement preserves both original scenario names verbatim, so archive does not drop them
- [x] 4.2 Re-read `specs/release-verification/spec.md` and confirm its package list matches the publish and verify sets in `.github/workflows/release.yml` exactly (7 packages, same names, same order)
- [x] 4.3 Run `openspec validate reconcile-parity-spec-drift --type change --json --no-interactive` and confirm it reports valid with no issues

## 5. Reach a clean strict-validation baseline (pre-gate check)

- [x] 5.1 Run `openspec validate --strict --specs --changes --json --no-interactive` and confirm 0 invalid items across all specs and in-flight changes
- [x] 5.2 Confirm the only remaining findings are INFO-level (the 12 long-requirement hints), which do not fail the build per `design.md` Risks, and record their spec ids for the follow-up note. Recorded: `cached-formula-value` (2), `conditional-formatting` (2), `date-cell-value` (1), `exceljs-parity` (2), `images` (2), `release-verification` (1), `rich-text` (3), `streaming-write-incremental` (1), `streaming-write-to-file` (1), `streaming-write-to-readable` (1), `streaming-xlsx` (5), `tables` (1) — 12 specs, 22 findings, all `Requirement text is very long (>500 characters)`

## 6. Add the CI gate (last, and only once §1–§5 are green)

- [x] 6.1 Add an `openspec validate --strict --specs --changes --json --no-interactive` step to `.github/workflows/ci.yml`, invoking the CLI via `npx @fission-ai/openspec@1.14.0` per `design.md` D4 — verify the version is pinned exactly, not `@latest`
- [x] 6.2 Place the step so it runs on both the `push` and `pull_request` triggers already configured on that workflow, and verify it is not gated behind an `if:` condition that would skip it
- [x] 6.3 Verify locally that the exact command from 6.1 exits zero against the current tree
- [x] 6.4 Confirm the step introduces no new CI secret or credential, per the `release-verification` OIDC requirement's no-long-lived-credential rule
- [ ] 6.5 Open a PR and verify the new step runs and passes in CI on all three matrix OSes (`ubuntu-22.04`, `macos-14`, `windows-2022`) — the validation is path-independent but the runner setup is not
