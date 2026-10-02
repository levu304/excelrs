# 007 — Manual tag-driven release

**Status:** current
**Recorded at:** v0.1.0 (June 2026)

## Decision

Releases are cut by pushing an annotated git tag. The tag drives GitHub Actions, which
builds the platform matrix, runs the smoke tests, and publishes.

## Context

The project has one maintainer. Release cadence is a deliberate choice, not a queue, and
publishing is irreversible. A human deciding to cut a version is the cheapest possible
control on an irreversible action.

## Alternatives considered

- **Automated publish on merge to main.** Rejected — no gate between "merged" and
  "published to npm".
- **npm publish from a laptop.** Rejected — not reproducible, and no OIDC trust chain.

## Current form

Tag-driven on `main`, with npm **trusted publishing** (OIDC) rather than a stored token.
Trusted publishing is specified in the `release-verification` capability; the tag trigger is
documented at the top of `CHANGELOG.md`.

## Sources

- `CHANGELOG.md:2` — "Release process: tag-driven main. `git tag -a vX.Y.Z -m "..."` then
  push tag."
- `.github/workflows/release.yml`.
- Rationale recovered from the v1.0.0 specification's one-line table entry.
