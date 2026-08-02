## Context

The release pipeline (`release.yml`) and the package manifest (`package.json`)
each enumerate exactly three platform targets via four separate hardcoded
mechanisms:

1. `build` job matrix (3 entries: target + os + npm_dir)
2. `package.json` `napi.targets` (3 Rust targets)
3. publish job — per-platform `package.json` heredocs (3 files written into
   `npm/*/package.json`)
4. publish job — `optionalDependencies` constructed from a 3-entry inline JS
   object, and a "Verify all 4 packages" loop that hardcodes the count.

No mechanism currently references an arm64 Linux target, so no
`@levu304/excelrs-linux-arm64-gnu` package exists. The existing linux target
uses `x86_64-unknown-linux-gnu` (gnu), not musl. See proposal.md for motivation.

## Goals / Non-Goals

**Goals:**

- Ship an `aarch64-unknown-linux-gnu` prebuilt `.node` binary that loads and
  round-trips a styled workbook on arm64 Linux.
- Register it in npm as `@levu304/excelrs-linux-arm64-gnu` under
  `optionalDependencies` so npm auto-selects it.
- Add a trusted-publisher (OIDC) configuration for the new package on npmjs.com.

**Non-Goals:**

- `linux-arm64-musl` static build. The issue says "ideally"; musl adds
  cross-linker/libc-static compile surface and is deferred.
- Changing any Rust/TypeScript public API or the `formula-eval` feature set.
- Cross-compiling the arm64 binary from the x86 runner.

## Decisions

### Decision 1 — Build on a native arm64 runner (not cross-compile from x86)

**Chosen:** add a matrix entry on GitHub's arm64 Linux runner
(`ubuntu-24.04-arm`, or `ubuntu-22.04-arm` — see Open Questions) running the
same `npx napi build --platform --release --features formula-eval --js native.js
--target aarch64-unknown-linux-gnu` flow as the other targets.

**Alternatives considered:**

- *Cross-compile from `ubuntu-22.04` with `cross`/qemu:* requires installing
  `aarch64-linux-gnu-gcc` + `libc6-dev:arm64`, a `[target.aarch64-unknown-linux-gnu]`
  cargo config with a custom linker, and either qemu emulation to run the test
  suite or skipping tests on that leg. More fragile; the repo has no existing
  `.cargo/config` or `cross` dependency. Rejected.
- *Native runner:* `actions/checkout` + `actions-rust-lang/setup-rust-toolchain`
  - `actions/setup-node` all work on arm64 runners today; `napi build` resolves
  the target natively. Builds and tests run on the same arch — no emulation.
  Lowest-surprise, mirrors how `aarch64-apple-darwin` is already built on
  `macos-14`.

### Decision 2 — gnu ABI (matching the existing linux target)

**Chosen:** `aarch64-unknown-linux-gnu`, producing
`@levu304/excelrs-linux-arm64-gnu`.

The existing linux target is `x86_64-unknown-linux-gnu`, so matching the glibc
ABI is consistent and least-surprising for the Docker/Graviton audience
(issue's primary pain points). musl would broaden the reach (Alpine/Pi OS) but
re-opens the linker/libc-static questions above and is explicitly deferred.

### Decision 3 — Publish as a distinct optional dependency (no main-binary change)

**Chosen:** arm64 ships as `@levu304/excelrs-linux-arm64-gnu`, selected by npm
via `os`/`cpu` fields in its `package.json`. The main package's
`optionalDependencies` gains one line; no fallback/`prebuild`-style binary
resolution is introduced. Mirrors the existing three packages exactly.

## Risks / Trade-offs

- [Runner image + pinned SHA] → The workflow pins specific action SHAs
  (`actions/checkout@34e1148…`). These pins work on x64; an arm64 runner image
  version must be confirmed to accept the same checkout setup-node versions.
  **Mitigation:** use the arm64 runner image and verify the checkout step works
  (already used for `macos-14` darwin-arm64, so the PINNED-action pattern is
  proven cross-platform here — just swap os for the linux arm64 image).
- [OIDC trusted publisher] → Adding a 4th platform package means npmjs.com needs
  a 4th trusted-publisher entry (github workflow + package name). If forgotten,
  `npm publish` for that package fails OIDC. **Mitigation:** the release-verification
  delta spec now enumerates the 5 packages; add the npm publisher config in the
  same change as the workflow.
- [Compile cost] → `formula-eval` is on for the release build already; enabling
  it for arm64 doubles formula-eval compile time on the matrix (each entry
  compiles once, so it's +1 leg, same as every other platform). No new
  trade-off, just one more `cargo`/napi compile.
- [musl deferred] → arm64 Alpine/Pi users still can't auto-load the binary and
  must source-build. **Mitigation:** documented non-goal; revisit as a follow-up
  if demand appears.

## Migration Plan

Additive, zero-break. No data migration and no API migration:

- Existing three platforms: unchanged binaries, unchanged `optionalDependencies`
  entries (still present).
- arm64 Linux consumers: no longer need `--platform=linux/amd64` emulation or a
  manual `npm rebuild`/`source build`.

## Open Questions

- Which arm64 runner image to pin: `ubuntu-24.04-arm` (newer) or
  `ubuntu-22.04-arm` (matches the x64 linux image for consistency)?
  Recommendation at implement time: match `ubuntu-22.04` family via
  `ubuntu-22.04-arm` if available, else `ubuntu-24.04-arm`. Either is fine; let
  CI stability decide.

## Out of scope (reinforced)

- True incremental streaming write (ADR-005) — unrelated to platform binaries.
- Formula engine expansion beyond the `formula-eval` `xlstream-parse` bridge.
- The `cell-mutation` directory under `openspec/specs/` has no spec file; this
  change does not touch cell-value semantics.
