# Design

## Context

PR #64 serialized rich text into `xl/sharedStrings.xml` (`<si><r>…</r></si>`)
and emitted cells as `t="s"` so Apple Numbers renders per-run fonts. The
dedup key for those runs is `SharedString::Rich(Vec<RichTextRun>)`, which
derives `Hash`/`Eq` over the **full** `Font` struct. `Font` gained a hand-rolled
`Hash` (f64 `to_bits()`) and a manual `Eq` in that PR, with a comment claiming
"no NaN keys" because `Font::validate` rejects non-finite `size`/`color`.

Two review findings show this is fragile:

- `color_tint` (and `color_theme`) are `#[napi(skip)]` — never exposed to JS,
  never rendered by `write_rich_run_xml` — yet they sit in the `Hash`/`Eq` key.
  `Font::validate` does **not** check `color_tint`, so the "validate guards the
  key" assumption is already broken (the NaN-hole is effectively unreachable
  because `color_tint` only comes from a finite XML `tint` attribute, but the
  coupling is wrong and the comment is misleading).
- `to_bits()` makes `+0.0` and `-0.0` hash differently while `Eq` treats them
  as equal — a `Hash`/`Eq` contract violation that causes missed dedupe.

Separately, the test suite round-trips through excelrs's own lenient reader,
which cannot catch a strict consumer (Apple Numbers) regression. And the
streaming writer (`src/stream.rs`) has its own `write_shared_strings(&[String])`
and no `RichText` variant, so any future streaming rich-text support could
silently diverge to `inlineStr`.

## Decision

**1. Project the key to rendered fields.** Introduce a small `RenderedRunKey`
struct holding only the fields `write_rich_run_xml` emits
(`name`, `size`, `bold`, `italic`, `underline`, `color`), with derived
`PartialEq`/`Eq`/`Hash`. `SharedString::Rich` keys on `Vec<RenderedRunKey>`
instead of `Vec<RichTextRun>`. `build_shared_strings` projects each
`RichTextRun` into a `RenderedRunKey`; `write_cell_xml` looks it up the same
way. `write_rich_run_xml` stays the single source of truth for *what is
rendered*, and the key is that exact projection.

**2. Remove the hand-rolled `Font` `Eq`/`Hash`.** Once the key no longer needs
`Font: Hash`/`Eq`, revert the `to_bits` `Hash` impl, the manual `Eq`, and the
`PartialEq` derive added in PR #64 from `src/model/style.rs`. Keep
`Font::validate` unchanged. Also delete the now-inaccurate "no NaN keys"
comment. This removes the fragile validate↔key coupling and the signed-zero
contract bug in one move (the `RenderedRunKey` `Hash` normalizes `size` signed
zero so `+0.0 == -0.0` hashes equally).

**3. Streaming guardrail.** Document on `StreamValue` / the streaming writer
that rich-text write, when added, MUST use the shared-string path (`t="s"`)
reusing `write_rich_run_xml`/`SharedString`, never `inlineStr`. Make the
streaming `write_shared_strings` `SharedString`-aware (share the same type and
key logic) so the two writers cannot diverge. Add a forward-looking unit test
asserting `t="s"` for streaming rich text, gated on the future `RichText`
variant.

**4. Confidence tests.** Add (a) a golden-file test asserting exact
`xl/sharedStrings.xml` + `xl/worksheets/sheetN.xml` for a known rich-text cell
(run `<rPr>` contents, `xml:space="preserve"`, `t="s"` + `<v>`); and (b) an
OOXML conformance smoke test — validate the generated workbook against the
OOXML XSD and/or open it headless in LibreOffice (`soffice --headless
--convert-to`). Keep `scripts/rich-text-repro.cjs` as the documented manual
Apple Numbers step.

## Alternatives

- **Keep full-struct key + three band-aids** (`validate_float(color_tint)`,
  normalize signed zero in `to_bits`, exclude unrendered fields from the key).
  Rejected: three narrow fixes vs one conceptual change; still couples the key
  shape to `Font`'s struct and still requires a "what gets validated" comment to
  stay correct.
- **Hash the rendered XML bytes directly** as the key. Considered but rejected:
  allocates a `String` per run for hashing. The parallel `RenderedRunKey`
  projection is lighter, unit-testable, and keeps the key obviously equal to
  what is written.

## Risks

- Changing the dedup key changes shared-string indices in output. Covered by
  the golden-file test, which asserts exact `sharedStrings.xml`.
- LibreOffice may be absent in some CI runners. Make the conformance test
  feature-gated / optional and document XSD validation as the fallback so CI
  does not hard-fail where LibreOffice is unavailable.
- No end-user-visible behavior change for normal rich text; output is
  byte-equivalent for finite, rendered-field runs. Dedup *improves* (theme/tint
  only differences now collapse).
