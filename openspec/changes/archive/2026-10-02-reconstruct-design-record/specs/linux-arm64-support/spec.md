# Spec Delta

## REMOVED Requirements

### Requirement: Release build matrix includes the aarch64-unknown-linux-gnu target

**Reason**: This requirement enumerated one matrix entry by target triple and npm directory.
It also made the capability's existence contingent on a single increment: the three platforms
present since the first release have no corresponding capability, so the corpus recorded when
a platform was added rather than what the platform guarantees.

**Migration**: The `Every build-matrix target produces a named native artifact` requirement in
`platform-targets` governs this target from the `release.yml` matrix, and applies to every
target including future ones.

### Requirement: linux-arm64 binary is published as an optional dependency

**Reason**: The requirement restated `optionalDependencies` and `napi.targets` entries in
requirement form for one target, so it would need amendment on every platform addition and
gave no coverage to the platforms that predate this capability.

**Migration**: The `Every build-matrix target is resolvable at install time` and
`The build matrix and declared targets agree` requirements in `platform-targets` state this
once for all targets.

### Requirement: Release job runs the arm64 smoke test

**Reason**: The functional smoke-test obligation was stated for one target only. The
assertions it specifies — styled round-trip, merge and row-style survival, and streaming
regression — are release-wide, and `release-verification` already states the styled and
streaming round-trips; scoping them per platform duplicated the guarantee without
strengthening it.

**Migration**: `Every build-matrix target runs the functional smoke test` in `platform-targets`
requires the smoke test for every target, including this one.
