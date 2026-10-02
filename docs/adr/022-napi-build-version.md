# 022 — `napi-build` in build-dependencies

**Status:** current (corrected — see *Correction*)
**Recorded at:** v0.1.0 (June 2026)

## Decision

Declare `napi-build` in `[build-dependencies]` so `build.rs` can call
`napi_build::setup()`.

## Context

Standard napi-rs scaffolding. The `build.rs` in this crate is a single call to
`napi_build::setup()`.

## Correction (added at v2.9.1)

**The version in the original decision was wrong.** It specified `napi-build = "3"`.
`Cargo.toml` carries **`napi-build = "2"`**.

This was not a mistake with consequences — the original decision's own parenthetical noted
that `napi-build = "2"` also works with napi v3, because the crate is scaffolding glue
rather than part of the FFI runtime. The v3 scaffold emits `"2"`. The *decision* (declare
napi-build, use the scaffold's version) is sound and is what the code does; only the number
in the title was aspirational.

Recorded as a correction rather than a silent fix, because a reader comparing this file
against `Cargo.toml` should see that they once disagreed and why they do not now.

## Sources

- `Cargo.toml:33` — `napi-build = "2"` under `[build-dependencies]`.
- `build.rs` — `napi_build::setup()`.
- OpenSpec change `reconstruct-design-record`, design D5.
