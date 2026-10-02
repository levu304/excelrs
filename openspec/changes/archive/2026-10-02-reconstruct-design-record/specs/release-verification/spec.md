# Spec Delta

## ADDED Requirements

### Requirement: Trusted publishing is configured for every published package

Every package published by the release workflow SHALL have a trusted-publisher configuration
on npmjs.com authorizing the `release.yml` workflow to perform `npm publish`. The set of
packages this applies to SHALL be the set the publish job publishes, as declared by
`release.yml`.

#### Scenario: Every published package can be trusted-published

- **WHEN** the release workflow publishes its package set
- **THEN** every package in that set SHALL have a trusted-publisher configuration
  authorizing the `release.yml` workflow

#### Scenario: A newly added platform package is covered

- **WHEN** a platform package is added to the publish job's package set
- **THEN** it SHALL have a trusted-publisher configuration without a specification change

## REMOVED Requirements

### Requirement: Trusted publishing is configured for the seven published packages

**Reason**: The requirement's name and body both enumerated the package set — a count and
seven explicit names — making a restatement of `release.yml` into a normative requirement.
#66 corrected the count from 5 to 7 in the body but the name and the enumeration both
remained, so the requirement still goes stale on the next platform addition. The name
itself carried the rot, which is why this is a removal and an addition rather than a
modification.

**Migration**: The `Trusted publishing is configured for every published package` requirement
added above states the same obligation against the publish job's declared package set, so
it holds without amendment as targets are added.

### Requirement: Release package set matches the release-verification spec

**Reason**: The requirement made this capability its own subject, so agreement could only be
checked by reading the specification against itself. Package-set agreement is now expressed
as an invariant anchored on `release.yml`, the artifact the publish job actually reads.

**Migration**: Rely on the `Every build-matrix target is resolvable at install time` and
`The build matrix and declared targets agree` requirements in the `platform-targets`
capability, which name `release.yml` as the source of truth.

### Requirement: Patch release SHALL follow existing release process

**Reason**: The requirement restated the package count inline and had already gone stale —
its scenario asserted a release publishes 5 packages, contradicting the 7-package set
recorded elsewhere in the same capability. The release process is already fully specified by
the requirements in this capability; a requirement that only points at its neighbours adds a
place to rot without adding a guarantee.

**Migration**: Patch releases continue to be governed by the smoke-test and
trusted-publishing requirements in this capability, which state their package obligations
without a count.
