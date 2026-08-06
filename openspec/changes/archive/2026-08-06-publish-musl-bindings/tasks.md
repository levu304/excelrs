## 1. Toolchain & local dev parity

- [x] 1.1 Build musl via `cargo-zigbuild` (`napi build --cross-compile`) with `-C target-feature=-crt-static` for dynamic musl linkage; `.cargo/config.toml` documents this and the `-C linker-plugin-lto=off` workaround for the rustc-1.97 / zig-0.14 (LLVM 21) bitcode mismatch. Does not affect gnu/darwin/windows builds.
- [x] 1.2 Add `x86_64-unknown-linux-musl` and `aarch64-unknown-linux-musl` to `package.json` `napi.targets` so local `napi build --platform` covers musl.

## 2. Release build matrix

- [x] 2.1 Add `linux-x64-musl` and `linux-arm64-musl` build matrix entries: `target: x86_64-unknown-linux-musl` / `aarch64-unknown-linux-musl` on `ubuntu-22.04` / `ubuntu-22.04-arm`, `npm_dir: linux-x64-musl` / `linux-arm64-musl`.
- [x] 2.2 Gate `musl-tools` install + `rustup target add *-musl` to the musl legs only (gnu/darwin/windows legs must run unchanged).
- [x] 2.3 Pass `--target ${{ matrix.target }}` to the `npx napi build ... --js native.js` command (uniform; no-op for gnu/darwin/windows since host==target).

## 3. Publish job packaging

- [x] 3.1 Add `npm/linux-x64-musl/package.json` and `npm/linux-arm64-musl/package.json` platform manifests via the existing "Create platform package.json files" heredoc block (`os: ["linux"]`, matching `cpu`, `main` = `excelrs.linux-*-musl.node`).
- [x] 3.2 Confirm the `for dir in npm/*/` publish loop already covers the new dirs (it globs all); add an explicit list entry only if the loop is narrowed.
- [x] 3.3 Add `@levu304/excelrs-linux-x64-musl` and `@levu304/excelrs-linux-arm64-musl` to the `optionalDependencies` rewrite block.
- [x] 3.4 Extend the "Verify all N packages on npm" loop from 5 to 7 packages (include both musl packages).

## 4. Verification

- [x] 4.1 CI: the two new musl matrix legs build and upload `excelrs.linux-*-musl.node` artifacts without failing the gnu/darwin/windows legs.
- [x] 4.2 Smoke test: the musl x64 binary loads in a Node.js process and round-trips a styled workbook (write + read back, assert style preserved).
- [x] 4.3 Smoke test: the musl arm64 binary loads on an arm64 host (static-link load on glibc arm64).
- [x] 4.4 `openspec validate --strict` passes for this change.
