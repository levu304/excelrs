## Context

PR #36 (`fix/typed-enum-declarations`) is ready to merge. It contains 3 review fixes (pattern fill, border styles, alignment vertical) on top of the typed enum declarations work. The branch is up to date and CI passed on the latest commit 228023e. The next step is to merge to main and cut a new release.

## Goals / Non-Goals

**Goals:**

- Merge PR #36 into `main`
- Trigger release workflow for new npm version
- Verify release tag and publish

**Non-Goals:**

- No code changes in this change — all code work is already done in PR #36
- No manual npm publish — use release-please automation

## Decisions

- **Merge strategy**: Rebase — keeps a linear history. The branch has 2 commits that are already clean.
- **Release automation**: Use existing release-please workflow (`.github/workflows/release.yml`). It handles CHANGELOG, version bump, git tag, GitHub release, and npm publish.
- **Version**: v2.3.0 — minor bump for the new typed enum declarations feature and OOXML fixes.

## Risks / Trade-offs

- **CI failure on main**: Low risk — CI passed on the branch at commit 228023e. Only risk is merge skew with parallel changes, but there are no changes on `main` since the PR was opened. Mitigation: monitor CI after merge.
- **Release-please race**: If release-please PR conflicts with the merge. Mitigation: use the automated release tools (`release_pr_merge`) which handle this.
- **npm publish failure**: Off chance if npm token is stale. Mitigation: verify publish in CI output.
