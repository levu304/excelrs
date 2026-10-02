# Spec Delta

## RENAMED Requirements

- FROM: `### Requirement: The pipeline runs the step that produces the published type transforms`
- TO: `### Requirement: The pipeline runs the step that produces the generated type transforms`

## MODIFIED Requirements

### Requirement: The pipeline runs the step that produces the generated type transforms

Every build the pipeline performs — continuous integration and release — SHALL run the same
post-build transformation of generated type declarations that a local build runs. The
transformed declarations are a build output consumed by type-checking; they are not part of
the published package, whose type declarations are hand-maintained and unaffected by this
transform.

#### Scenario: CI and release apply the same transform as a local build

- **WHEN** the pipeline builds the native addon for continuous integration or for release
- **THEN** it SHALL invoke the same post-build type-declaration transformation that the
  project's local build script invokes

#### Scenario: A build that skips the transform is not equivalent

- **WHEN** a build omits the post-build type-declaration transformation
- **THEN** its generated type declarations SHALL be treated as not equivalent to a local
  build's, and a check that reads the generated declarations SHALL fail

#### Scenario: The transform does not reach a consumer

- **WHEN** the package is packed for publication
- **THEN** the transformed generated declarations SHALL NOT be among the published files,
  and the published type declarations SHALL be unchanged by the transform