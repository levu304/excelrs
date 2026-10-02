# Spec Delta

## MODIFIED Requirements

### Requirement: Formula evaluation is opt-in via Cargo feature

The system SHALL gate the formula evaluation code behind a Cargo feature
named `formula-eval`. When the feature is disabled (default), the evaluation
surface SHALL remain present but inert: `recalculate` SHALL perform no
evaluation and `cachedValue` SHALL remain `null` for formula cells. The
internal evaluator type SHALL NOT be exposed to the JS API in any build.

#### Scenario: Evaluation API absent without feature

- **WHEN** the crate is built without `--features formula-eval`
- **THEN** the evaluator implementation and its dependencies SHALL be absent from the
  build, no formula SHALL be evaluated, and every formula cell's `cachedValue` SHALL
  remain `null`

#### Scenario: The evaluation surface stays callable without the feature

- **WHEN** the crate is built without `--features formula-eval`
- **THEN** `Cell::cachedValue`, `Worksheet::recalculate`, and `Workbook::recalculate`
  SHALL still be present in the public API and callable, and the internal evaluator type
  SHALL NOT appear in the generated JS type declarations

## ADDED Requirements

### Requirement: Published artifacts compile in formula evaluation

Every artifact the release workflow publishes SHALL be compiled with the
`formula-eval` feature enabled, so a package consumer receives working
recalculation without a build-time opt-in.

#### Scenario: Release builds enable the feature

- **WHEN** the release workflow builds the native addon for any published target
- **THEN** that build SHALL enable the `formula-eval` feature

#### Scenario: A published package evaluates formulas

- **WHEN** a consumer installs the package and calls `recalculate` on a formula cell
- **THEN** the cell's `cachedValue` SHALL be populated, with no Cargo feature opt-in by
  the consumer

#### Scenario: Opt-in describes a source build, not the shipped package

- **WHEN** the crate is built from source with its default Cargo feature set
- **THEN** recalculation SHALL be inert, and this opt-in SHALL NOT describe what the
  release workflow publishes

### Requirement: The local build matches the pipeline's feature set

The project's local build scripts SHALL enable the same Cargo features that the
continuous-integration and release builds enable, so a developer running the
documented build-then-test sequence exercises the same artifact the pipeline
tests.

#### Scenario: The documented local loop passes

- **WHEN** a developer runs the local build script and then the JS test suite
- **THEN** the test suite SHALL pass without requiring an extra flag the documentation
  does not mention

#### Scenario: The build scripts agree with CI

- **WHEN** the local build scripts and the CI build step are compared
- **THEN** both SHALL enable the same set of Cargo features