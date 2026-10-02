# Spec Delta

## MODIFIED Requirements

### Requirement: The local build matches the pipeline's feature set

The project's local build scripts and the continuous-integration and release builds SHALL
enable the same set of Cargo features, so a developer running the documented
build-then-test sequence exercises the same artifact the pipeline tests. A check SHALL compare
the feature sets declared by the local build script against those declared by the continuous-
integration and release build steps, and the CI pipeline SHALL run that check.

#### Scenario: The documented local loop passes

- **WHEN** a developer runs the local build script and then the JS test suite
- **THEN** the test suite SHALL pass without requiring an extra flag the documentation
  does not mention

#### Scenario: The build scripts agree with CI

- **WHEN** the local build scripts and the CI build step are compared
- **THEN** both SHALL enable the same set of Cargo features

#### Scenario: A dropped feature flag is detected before release

- **WHEN** the local build script's declared Cargo features differ from the CI or release build
  step's declared features
- **THEN** the check SHALL fail, naming both the file and the features that differ

#### Scenario: The check runs in CI

- **WHEN** the CI pipeline runs
- **THEN** it SHALL execute the feature-set comparison and fail the build on any difference