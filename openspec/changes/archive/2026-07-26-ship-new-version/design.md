## Context

Current version is 2.2.0 (shipped 2026-07-20). Since then, three changes have landed on main:

1. **Style setter type safety** (2026-07-26, `style-setter-type-safety`) — Refactors Rust setter parameter from `serde_json::Value` to `Option<Style>` so napi-rs generates typed `Style | null | undefined` instead of `any`. No runtime behavior change.
2. **Row.getCell() value lost on cloned Row** (2026-07-26, `row-getcell-value-lost`) — Changes `Row::cells` from `HashMap<u32, Cell>` to `Arc<Mutex<HashMap<u32, Cell>>>` so mutations on a cloned row propagate back. Correctness fix.
3. **Fix review findings** (2026-07-26, `fix-review-findings`) — Removes redundant `map_err` calls, extracts shared `apply_style` helper to `style.rs`, adds missing edge-case tests. Internal cleanup.

All three are fully coded, reviewed, and committed. None has been released.

The release process is tag-driven: push an `v*` tag → CI Release workflow builds native binaries for 3 platforms, publishes 4 npm packages, and creates a GitHub Release.

## Goals / Non-Goals

**Goals:**

- Publish v2.2.1 to npm with all three unreleased changes
- Update CHANGELOG.md with accurate, user-facing descriptions
- Verify the release end-to-end (CI build, npm publish, GitHub Release)
- Establish a precedent for lightweight patch releases

**Non-Goals:**

- No code changes or feature additions
- No changes to the CI/CD pipeline or release workflow
- No version bumps in `Cargo.lock` or platform packages (generated during CI)

## Decisions

1. **Patch bump (2.2.0 → 2.2.1) not minor (2.3.0).** The changes are a correctness bug fix, a type-only tightening (no behavioral API change), and internal code cleanup. Per semver, all three qualify as patch. A minor bump would signal new features, which misrepresents the scope.

2. **Manual version + changelog commit, not automated.** The existing process is manual (commit version bumps + changelog, then tag). Introducing release-please or similar automation is out of scope for this patch release. Document the steps for consistency.

3. **CHANGELOG entry format follows existing pattern.** Each change gets a `### Fixed` or `### Changed` section with a terse, user-facing description and archived change reference, matching the v2.2.0 entry style.

## Risks / Trade-offs

| Risk | Mitigation |
| ------ | ------------ |
| **Tag pushed before CI on main passes** | Run CI on main first, confirm green, then tag. The Release workflow also runs tests before publishing. |
| **npm publish flake (registry lag)** | The workflow already has 5-retry verification for each package. No action needed. |
| **Platform build failure on one target** | The matrix builds are independent — the workflow would fail that target only. Manual retry after root-cause fix. |
| **Missing CHANGELOG detail** | The archived proposals have full context. Reference them in the changelog for traceability. |
