# Proposal: publish-musl-bindings

## Why

`@levu304/excelrs` ships prebuilt native addons for glibc/darwin/msvc only (no `linux-*musl`). The addon is compiled against glibc and cannot load under musl libc, so every consumer on Alpine/musl (distroless-musl, `alpine`, etc.) fails at `require()` time. Downstream projects are forced off small Alpine bases onto glibc images (e.g. `node:bookworm-slim` / `distroless/nodejsNN-debian12`) to avoid a source compile, increasing image footprint and pinning them to debian instead of alpine. This is purely an artifact/platform-packaging gap — the Rust code has no glibc dependency at the source level (no `openssl`, `libz-sys`, or other C/FFI libs in `Cargo.toml`), so the same N-API ABI builds cleanly against musl.

## What Changes

- **New capability `musl-support`**: produce `linux-x64-musl` and `linux-arm64-musl` native addon packages that npm resolves automatically on musl Linux hosts.
- **Release matrix** (`.github/workflows/release.yml`): add two build matrix entries targeting `x86_64-unknown-linux-musl` and `aarch64-unknown-linux-musl` (on the existing `ubuntu-22.04` x64 and `ubuntu-22.04-arm` runners respectively), each producing `excelrs.linux-{x64,arm64}-musl.node` and uploading under `npm_dir` `linux-x64-musl` / `linux-arm64-musl`.
- **Build toolchain**: on the musl legs, install `musl-tools` (provides `musl-gcc`) and `rustup target add …-musl`, and pass `--target` to `napi build` (driven by a new `matrix.target` so the existing legs are unaffected).
- **`package.json`**: add `x86_64-unknown-linux-musl` and `aarch64-unknown-linux-musl` to `napi.targets`, plus a `cargoExtraArgs` entry mapping them to `musl-gcc`; add both to `optionalDependencies` via the publish job's package.json rewrite.
- **Publish job**: hand-craft `npm/linux-*-musl/package.json` platform manifests (mirroring the existing gnu ones); extend the "Create platform package.json files", "Publish platform packages", and "Verify all N packages on npm" loops to include the two new packages (7 total: 5 existing + 2 musl).

**No runtime or API change.** The addon loads statically linked (musl libc baked in at build time), so it runs on any musl host regardless of that host's libc version. No change to `index.js`, `index.d.ts`, the napi glue (`apply-glue.cjs`), or any Rust source. The release-time `--features formula-eval` flag is preserved unchanged.

## Capabilities

### New Capabilities

- `musl-support`: the system produces, publishes, and resolves static musl native addon packages (`@levu304/excelrs-linux-x64-musl`, `@levu304/excelrs-linux-arm64-musl`) that load on Alpine/musl Linux without a source build. See `specs/musl-support/spec.md`.

### Modified Capabilities

- none — the existing `linux-arm64-support` (gnu) requirement is unchanged; musl is additive.

## Impact

- `.github/workflows/release.yml` — build matrix + publish loops (CI/release surface).
- `package.json` — `napi.targets` + `optionalDependencies`.
- No effect on `src/` (Rust core), `index.js`, `native.d.ts`, or the napi glue.
- External: consumers on Alpine/musl gain an auto-resolved binary; no migration needed (additive optional dependencies).
- Release: total platform packages goes from 5 to 7. The "Verify all packages on npm" step must count 7.
