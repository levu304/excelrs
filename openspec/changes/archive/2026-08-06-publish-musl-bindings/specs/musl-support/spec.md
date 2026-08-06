## Purpose

Produce, publish, and resolve static musl native addon packages (`linux-x64-musl` and `linux-arm64-musl`) so consumers on Alpine/musl Linux hosts can load `@levu304/excelrs` without a source build or pinning to a glibc base image.

## ADDED Requirements

### Requirement: Release build matrix includes musl targets

The `release.yml` build matrix SHALL include entries targeting `x86_64-unknown-linux-musl` and `aarch64-unknown-linux-musl`. Each entry SHALL build the native addon for that target, and SHALL upload an artifact named `excelrs.linux-x64-musl.node` (resp. `excelrs.linux-arm64-musl.node`) under npm directory `linux-x64-musl` (resp. `linux-arm64-musl`).

#### Scenario: New musl matrix entry builds native muscle binary

- **WHEN** the release build job runs with `target: x86_64-unknown-linux-musl` (and separately `aarch64-unknown-linux-musl`)
- **THEN** the job SHALL produce `excelrs.linux-x64-musl.node` (resp. `excelrs.linux-arm64-musl.node`) and upload it as the `linux-x64-musl` build artifact (resp. `linux-arm64-musl`)

### Requirement: musl binaries published as optional platform packages

The publish job SHALL create `@levu304/excelrs-linux-x64-musl` and `@levu304/excelrs-linux-arm64-musl` platform packages (each with `os: ["linux"]`, `cpu` matching the triple) and npm-publish them. The main `@levu304/excelrs` package SHALL declare both in `optionalDependencies`, and the `package.json` `napi.targets` array SHALL include `x86_64-unknown-linux-musl` and `aarch64-unknown-linux-musl` so local `napi build` covers them.

#### Scenario: musl Linux consumer installs musl binary

- **WHEN** a consumer on a musl Linux host (e.g. Alpine) runs `npm install @levu304/excelrs@<version>`
- **THEN** npm SHALL resolve `@levu304/excelrs-linux-x64-musl` (or `linux-arm64-musl` for arm64) as the native binary and `require('@levu304/excelrs')` SHALL succeed without a source compile

### Requirement: musl binary loads and functions correctly

The release SHALL verify each musl binary loads and round-trips a workbook (write + read back) in a musl environment. Because a statically-linked musl addon carries its own libc, it SHALL load in any Node.js process regardless of host libc.

#### Scenario: musl binary round-trips a styled workbook

- **WHEN** the publish job loads `@levu304/excelrs-linux-x64-musl` in a musl-capable Node.js process and builds, writes, then reads back a styled workbook
- **THEN** `require('@levu304/excelrs')` SHALL succeed, the write SHALL produce a non-empty XLSX, and the read-back SHALL preserve a cell style (`font.bold` and `fill.foreground`)

## REMOVED Requirements

(none — this is an additive capability)

## MODIFIED Requirements

(none — no existing requirement changes; the gnu variants in `linux-arm64-support` are untouched)
