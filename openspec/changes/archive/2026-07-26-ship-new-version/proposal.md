## Why

Ship v2.2.1 as a patch release containing three landed-but-unreleased changes since v2.2.0: the `Row.getCell()` value-lost-on-clone bug fix (correctness), the style-setter TypeScript type-safety tightening (DX), and post-archive review-findings cleanup (internal quality). These have all been coded, reviewed, and merged into main but never tagged/published — an unreleased fix means users hitting the cloned-Row cell-value bug have no workaround.

## What Changes

- **Version bump**: `package.json` 2.2.0 → 2.2.1, `Cargo.toml` 2.2.0 → 2.2.1 (patch)
- **CHANGELOG.md**: Add `[2.2.1]` entry documenting the three changes
- **Git tag**: `v2.2.1` pushed to trigger the Release workflow
- **CI/CD**: Release workflow builds/publishes all 4 npm packages + creates GitHub Release

No code changes. The diff is version bumps + changelog only.

## Capabilities

### New Capabilities

None — release-engineering only, no new features.

### Modified Capabilities

- `release-verification`: The release-verification spec defines the release process. Recording the v2.2.1 release steps confirms the process works and provides a precedent for future patch releases.

## Impact

- **`package.json`**: version field
- **`Cargo.toml`**: version field
- **`CHANGELOG.md`**: new entry
- **CI**: Release workflow on tag push (no changes to workflow files)
- **npm**: 4 packages published (`@levu304/excelrs`, `-darwin-arm64`, `-linux-x64-gnu`, `-win32-x64-msvc`)
- **GitHub**: Release created with auto-generated notes
