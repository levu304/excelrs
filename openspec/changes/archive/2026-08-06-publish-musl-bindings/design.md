## Context

The release pipeline (`.github/workflows/release.yml`) is hand-rolled, not `napi publish`. It runs a 4-entry build matrix where each runner builds the addon for its own host triple (`npx napi build --platform --release --features formula-eval --js native.js`) with **no `--target` flag today** (host == target). Each leg uploads `excelrs.<npm_dir>.node` as an artifact under a `linux-x64-gnu` / `linux-arm64-gnu` / `darwin-arm64` / `win32-x64-msvc` directory. The `publish` job then downloads all artifacts into `npm/`, hand-crafts a `package.json` per platform, npm-pushes each, rewrites `optionalDependencies` on the main package by hand, and re-verifies all 5 packages.

`Cargo.toml` has **no C/FFI dependencies** that would block static musl linking: `napi` (no default libc), `zip` (deflate via `miniz_oxide`, Rust), `quick-xml`, `calamine`, `chrono` (pure-Rust default, no `clock`/`wasmbind`), `tokio`, `futures-core`. `cargo build` with `--target *-musl` therefore needs only a musl linker, not a system libc-dev.

## Goals / Non-Goals

**Goals:**

- Publish `linux-x64-musl` and `linux-arm64-musl` platform packages that npm auto-resolves on Alpine/musl.
- Reuse existing runners (`ubuntu-22.04`, `ubuntu-22.04-arm`) — no new OS images.
- Statically linked addon (musl libc baked in) → loads on any musl host, libc-version-independent.

**Non-Goals:**

- Switching the publish flow to `napi publish` (consistency with current hand-rolled approach).
- Any runtime/API/`index.js`/napi-glue change.
- musl variants for darwin/windows (no such platform exists).

## Decisions

1. **Reuse gnu runners for musl legs.** `x86_64-unknown-linux-musl` builds on `ubuntu-22.04` (x64); `aarch64-unknown-linux-musl` builds on `ubuntu-22.04-arm` (arm64). Rationale: least blast radius — same hosts already used for the gnu arm64 leg; no new runner OS. (Alternative: a true Alpine runner — rejected: GitHub-hosted runners don't ship Alpine, and static musl already solves the "musl host" problem.)

2. **Static musl linking via `musl-tools`.** Install `musl-tools` (provides `musl-gcc`) + `rustup target add *-musl`, and drive cargo with a per-target linker. Decision: ship a `.cargo/config.toml` (`[target.<triple>] linker = "musl-gcc"`) rather than inline `RUSTFLAGS`, because the config is reusable by contributors doing local musl builds and survives shell-quoting differences. (Alternative: inline `RUSTFLAGS` only in CI — rejected: diverges local/CI and is easy to forget.)

3. **Pass `--target ${{ matrix.target }}` to `napi build` uniformly.** For the existing gnu/darwin/windows triples host==target, so this is a no-op for them; for musl triples it's required. Single change, zero regression for existing legs.

4. **Mirror the existing hand-rolled publish flow.** Add two platform `package.json` manifests (`linux-x64-musl`, `linux-arm64-musl`) to the "Create platform package.json files" heredoc block; add both to the `for dir in npm/*/` publish loop and the "Verify all N packages on npm" loop (5 → 7). Rationale: consistency with current machinery; least surprise for the maintainer who wrote it this way. (Alternative: migrate to `napi publish` — rejected: out of scope, large blast radius.)

5. **musl smoke-test via static load on the existing ubuntu publish host.** A statically-linked musl cdylib has no dynamic libc dependency, so it `dlopen`s in any Node.js process of the same CPU arch regardless of host libc. So the publish job (ubuntu x64 glibc) can load `excelrs.linux-x64-musl.node` directly. For arm64-musl, an arm64 glibc Node host (`ubuntu-22.04-arm`) loads the static arm64-musl binary the same way — no cross-arch emulation needed. (Alternative: an Alpine-based smoke job — deferred; static-load check is sufficient and cheaper.)

6. **Add musl triples to `package.json` `napi.targets`.** Drives local `napi build --platform` to produce musl artifacts for contributors. Mirrors the gnu/darwin/windows entries already present.

### npm platform naming

`@napi-rs/cli` maps `x86_64-unknown-linux-musl` → `linux-x64-musl` and `aarch64-unknown-linux-musl` → `linux-arm64-musl`, matching the `npm_dir` values used throughout `release.yml`. So artifact filenames and directory names line up automatically; no custom suffix logic needed.

## Risks / Trade-offs

- **[musl linker not auto-detected]** cargo may fall back to gcc/glibc or fail. → Mitigation: `.cargo/config.toml` pins `musl-gcc` per target; CI asserts the build emits a musl artifact (size/artifact name).
- **[arm64 smoke on arm64 glibc host]** assumes static linkage truly has zero dynamic libc symbols. → Mitigation: `ldd --version`-equivalent check absent on static binary; a failed `require()` in smoke test fails the release loudly. Acceptable for a static build.
- **[\n] package.json rewrite in publish job is hand-maintained** — adding 2 packages means 2 more hand-written entries; easy to drift at release. → Mitigation: a single verify loop (already present) asserts all 7 packages are on npm; `optionalDependencies` is rewritten in the same heredoc block as today. No new complexity.
- **[LTO + strip=symbols in Cargo.toml]** applies to musl too — fine; musl static binaries stay small. No change needed.

## Migration Plan

None required. Additive only. Existing glibc/darwin/msvc consumers are unaffected. To roll back: remove the 2 matrix entries, the 2 musl triples in `napi.targets`, and the 2 `optionalDependencies` (plus delete the 2 platform `package.json` blocks). No migration for consumers — npm simply gains the musl resolution it previously lacked.

## Open Questions

- **Q1 (defer):** Should platform packages carry npm `libc` metadata (`"libc": ["musl"]`) in `package.json` for precise resolution? npm supports `libc` in addition to `os`/`cpu`. Currently gnu packages only set `os`/`cpu`. Decision deferred to tasks: keep parity with existing gnu packages (no `libc` field) for v1; can add later if needed. This does NOT change the spec.
- **Q2 (defer):** Should arm64-musl be smoke-tested on a real musl kernel rather than via static-load-on-glibc? Defer to v1.1; static-load check is the pragmatic first bar. This does NOT change the spec.
