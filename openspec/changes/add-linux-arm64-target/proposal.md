## Why

excelrs ships prebuilt native `.node` binaries for only three platforms
(`aarch64-apple-darwin`, `x86_64-unknown-linux-gnu`, `x86_64-pc-windows-msvc`).
There is **no** `linux-arm64` build, so every aarch64-Linux host — Apple Silicon
Macs in an arm64 Docker image, AWS Graviton, Raspberry Pi 4/5 — fails to load the
package with `Cannot find native binding`. The only workaround today is forcing
`x86_64` emulation (`FROM --platform=linux/amd64`), which blocks arm64 hosts
entirely and pays an emulation/CPU penalty on every call. Issue #53 tracks
this. Adding the target is a build/release packaging change: the API surface is
unchanged, only a new platform binary ships.

## What changes

1. Add `aarch64-unknown-linux-gnu` to the release build matrix (job) in
   `.github/workflows/release.yml`, mapped to `npm_dir: linux-arm64-gnu`.
2. Append `"aarch64-unknown-linux-gnu"` to `napi.targets` and the `optionalDependencies`
   construction in `package.json`.
3. Emit a 4th platform `package.json` template (`@levu304/excelrs-linux-arm64-gnu`)
   in the release publish job, alongside the existing three.
4. Bump the post-publish "verify all N packages on npm" loop from 4 → 5 and add
   the new arch package to the list.

No Rust, TypeScript, or public-API changes. `formula-eval` feature flags are
unchanged in the release build.

**Capability:** `linux-arm64-native` (new) — the package ships an arm64 Linux binary
that loads and runs on `aarch64-unknown-linux-gnu` without emulation.

## Capabilities

### New Capabilities

- `linux-arm64-support`: Build, publish, and smoke-test an `aarch64-unknown-linux-gnu` prebuilt binary. Covers the build-matrix entry, the `optionalDependencies` registration, and the load+round-trip assertion on arm64.

### Modified Capabilities

- `release-verification`: The published-package enumeration changes from "four packages" to "five packages" (adds `@levu304/excelrs-linux-arm64-gnu`), the trusted-publisher OIDC scenario list grows by one package, and the patch-release "all 4 npm packages" statement becomes "all 5". A delta spec updates these numeric/list references.

## Impact

- `.github/workflows/release.yml` — +1 matrix entry, +1 platform `package.json` heredoc, +1 `optionalDependencies` line, loop bound 4→5.
- `package.json` — `napi.targets` +1 entry; `optionalDependencies` (constructed dynamically in publish job) +1 entry.
- A new GitHub Actions runner image is required to natively build for arm64 (see design.md Decisions — the cross-compile alternatives are rejected there).
- No impact on consumers on the other three platforms; the new package is opt-in via `optionalDependencies` (npm selects only the matching platform).

## Non-goals

- musl static build (`linux-arm64-musl`). The issue asks "ideally" but the existing
  linux target is gnu and the lowest-surprise path matches it. musl is deferred
  (see design.md Non-Goals + Risks).
- Cross-compilation from the x86 runner. Rejected: requires a cross linker +
  aarch64 libc-dev which is more fragile than a native arm64 runner
  (see design.md).
