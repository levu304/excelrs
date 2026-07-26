## 1. Version Bump

- [x] 1.1 Bump version in `package.json` from 2.2.0 to 2.2.1
- [x] 1.2 Verify `Cargo.toml` version is unchanged (Rust crate version is independent)

## 2. Changelog

- [x] 2.1 Add `[2.2.1]` entry to `CHANGELOG.md` with three items
- [x] 2.2 Add `[2.2.1]` comparison link at bottom of CHANGELOG.md
- [x] 2.3 Add `[Unreleased]` comparison link (none existed) and backfill missing 2.x links

## 3. Commit and Tag

- [x] 3.1 Commit with message: `feat(release): v2.2.1 — Row.clone cell fix + style-setter type safety`
- [x] 3.2 Create git tag: `git tag -a v2.2.1 -m "v2.2.1"`
- [x] 3.3 Push commit and tag: `git push && git push --tags`

## 4. Release Verification

- [x] 4.1 Confirm CI Release workflow triggered on `v2.2.1` tag — run 30191929719
- [x] 4.2 Verify all 4 npm packages publish successfully — all at 2.2.1
- [x] 4.3 Verify functional smoke test passes in CI — release passed (success)
- [x] 4.4 Confirm GitHub Release created with auto-generated notes
- [x] 4.5 Archive this change execution
