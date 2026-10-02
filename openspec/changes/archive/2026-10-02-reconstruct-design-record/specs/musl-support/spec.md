# Spec Delta

## REMOVED Requirements

### Requirement: Release build matrix includes musl targets

**Reason**: This requirement enumerated two specific musl matrix entries by target triple and
npm directory. Restating a build matrix inside a requirement goes stale whenever the matrix
changes, and the two targets it names are already covered by the matrix-wide invariant in the
`platform-targets` capability.

**Migration**: The `Every build-matrix target produces a named native artifact` requirement in
`platform-targets` governs musl matrix entries from the `release.yml` matrix itself, with no
amendment needed when targets are added or removed.

### Requirement: musl binaries published as optional platform packages

**Reason**: This requirement enumerated musl package names, `optionalDependencies` entries,
and `napi.targets` values — a restatement of the publish configuration in requirement form.
The same obligation is now stated once for every build-matrix target.

**Migration**: The `Every build-matrix target is resolvable at install time` and
`The build matrix and declared targets agree` requirements in `platform-targets` cover musl
platform packages identically to every other target.

### Requirement: musl binary loads and functions correctly

**Reason**: The load-and-verify obligation was stated only for musl, which made it the only
target with an explicit correctness guarantee and left the other five unstated. The
non-obvious musl-specific constraint it recorded — that a musl addon must link musl
dynamically, because a fully static musl binary segfaults on load — is genuine platform
knowledge, not an inventory, and is preserved in `platform-targets`.

**Migration**: Smoke-test coverage is now stated for every build-matrix target by
`Every build-matrix target runs the functional smoke test`; the dynamic-linking constraint
survives as the musl scenario in `platform-targets`.
