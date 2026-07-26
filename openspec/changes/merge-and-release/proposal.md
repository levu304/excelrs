## Why

PR #36 (`fix/typed-enum-declarations`) includes 3 completed review fixes (pattern fill, border styles, alignment vertical) that are committed and pushed. These fixes need to land on `main` via merge, followed by a new release to publish the changes to npm.

## What Changes

- Merge PR #36 into `main`
- Run release-please to cut a new version (v2.3.0)
- Publish updated npm package with the OOXML fixes

## Capabilities

### New Capabilities

- `merge-pr`: Merge PR #36 and verify CI passes on main
- `release`: Run release-please publish workflow, verify tag/npm publish

### Modified Capabilities
<!-- No existing capability specs are changing -->

## Impact

- Git: PR #36 branch merged, then deleted
- npm: New version published with FillKind fix, border style variants, alignment fix
- No API changes beyond what's already in PR #36
