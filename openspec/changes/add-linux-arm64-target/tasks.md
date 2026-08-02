## 1. Build matrix

- [x] 1.1 Add `aarch64-unknown-linux-gnu` matrix entry to the `build` job in `release.yml` (`target: aarch64-unknown-linux-gnu`, `os: ubuntu-22.04-arm` or `ubuntu-24.04-arm`, `npm_dir: linux-arm64-gnu`), mirroring the existing three entries.
- [x] 1.2 Verify `npx napi build --platform --release --features formula-eval --js native.js` resolves `--target aarch64-unknown-linux-gnu` natively on the arm64 runner (no cross-linker config).

## 2. Package manifest

- [x] 2.1 Append `"aarch64-unknown-linux-gnu"` to the `napi.targets` array in `package.json`.

## 3. Publish job (platform packages)

- [x] 3.1 Add a `npm/linux-arm64-gnu/package.json` heredoc in the `Create platform package.json files` step: name `@levu304/excelrs-linux-arm64-gnu`, `os: ["linux"]`, `cpu: ["arm64"]`, `main: excelrs.linux-arm64-gnu.node`.
- [x] 3.2 Add `@levu304/excelrs-linux-arm64-gnu` to the `optionalDependencies` JS construction blob in the publish step.

## 4. Publish verification

- [x] 4.1 Update the "Verify all N packages on npm" loop: bump the count from 4 to 5 and add `@levu304/excelrs-linux-arm64-gnu` to the iteration list.

## 5. npm trusted publisher

- [ ] 5.1 Add a GitHub-trusted-publisher entry on npmjs.com for `@levu304/excelrs-linux-arm64-gnu` (workflow: `release.yml`, owner/repo: `levu304/excelrs`, publish access) — *external registry step, not a repo edit.*

## 6. Smoke + regression

- [ ] 6.1 On the next `v*` tag push, confirm the arm64 build job produces `excelrs.linux-arm64-gnu.node`, uploads the `linux-arm64-gnu` artifact, publishes all 5 packages, and the functional smoke test passes for the arm64 binary (style + merge + row-style round-trip).
- [ ] 6.2 Confirm the three existing platforms still publish and smoke-test unchanged (no `optionalDependencies`/`napi.targets` regression).
