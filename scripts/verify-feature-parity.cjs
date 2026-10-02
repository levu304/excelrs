#!/usr/bin/env node
// scripts/verify-feature-parity.cjs
// Guards one invariant, stated by openspec/specs/formula-eval:
// "The local build matches the pipeline's feature set".
//
// That requirement has been true by hand and false by accident. The local `build`
// script once omitted `--features formula-eval` entirely, so a developer running
// `pnpm build && pnpm test` exercised a binary the pipeline never tests. Nothing
// compared the two, and `openspec validate` reports the corpus clean either way.
//
// Scope: `napi build` invocations only. `--features` also appears on `cargo clippy`
// and `cargo test` lines, which are deliberately a different question (which feature
// set the Rust test suite exercises). Folding those in would make the comparison
// about the wrong thing.
//
// ponytail: ceiling — this matches the `--features` token with a regex rather than
// parsing the workflow YAML, so it has no YAML dependency. A flag written in a form
// this regex does not match (e.g. `--features="a,b"` on a folded line it cannot see)
// reads as "no features declared", which fails the comparison loudly rather than
// passing silently. That is the safe direction to be wrong in. Upgrade path if a
// legitimate invocation shape appears: add it to FEATURES_RE.

const fs = require('fs')
const path = require('path')

const ROOT = path.resolve(__dirname, '..')
const PKG = path.join(ROOT, 'package.json')
const WORKFLOW_DIR = path.join(ROOT, '.github', 'workflows')

// Matches `--features a,b` / `--features a b` / `--features="a,b"`, capturing
// everything up to the next flag so a trailing `--js native.js` is not swallowed.
const FEATURES_RE = /--features[=\s]+([^\n]*)/g

// A workflow step begins at `- name:` or `- uses:` under the steps list. Slicing at
// those offsets keeps each chunk scoped to one step so a `napi build` in one step
// cannot borrow the `--features` of an adjacent `cargo test` step.
const STEP_RE = /\n\s{6}-\s+(?:name|uses):\s*([^\n]*)/g

function parseFeatures(text) {
  const found = new Set()
  for (const m of text.matchAll(FEATURES_RE)) {
    // Cut at the next flag, then split on whitespace and commas.
    const value = m[1].split(/\s--/)[0]
    for (const raw of value.split(/[\s,]+/)) {
      const feature = raw.replace(/^["']|["']$/g, '').trim()
      if (feature) found.add(feature)
    }
  }
  return found
}

const invocations = []

// package.json: each script whose command actually invokes `napi build`.
const pkg = JSON.parse(fs.readFileSync(PKG, 'utf8'))
for (const [name, command] of Object.entries(pkg.scripts || {})) {
  if (!/\bnapi\s+build\b/.test(command)) continue
  invocations.push({
    where: `package.json script "${name}"`,
    features: parseFeatures(command),
  })
}

// Workflows: each step whose body invokes `napi build`.
const workflows = fs.existsSync(WORKFLOW_DIR)
  ? fs.readdirSync(WORKFLOW_DIR).filter((f) => f.endsWith('.yml') || f.endsWith('.yaml'))
  : []
for (const file of workflows.sort()) {
  const text = fs.readFileSync(path.join(WORKFLOW_DIR, file), 'utf8')
  // Slice per step, keeping the step's own `- name:` text so a failure names the
  // step a reader can find in the file rather than an opaque block index.
  const starts = [...text.matchAll(STEP_RE)]
  starts.forEach((m, i) => {
    const chunk = text.slice(m.index, starts[i + 1] ? starts[i + 1].index : undefined)
    if (!/\bnapi\s+build\b/.test(chunk)) return
    const label = m[1].trim().replace(/^["']|["']$/g, '')
    invocations.push({
      where: `.github/workflows/${file}${label ? ` step "${label}"` : ''}`,
      features: parseFeatures(chunk),
    })
  })
}

if (invocations.length === 0) {
  console.error(
    '[verify-feature-parity] FAILED\n\n' +
      '  - No `napi build` invocation was found in package.json or .github/workflows/.\n' +
      '    This check has nothing to compare and would pass vacuously.\n'
  )
  process.exit(1)
}

const failures = []

// The reference set is the first local build script; every other invocation must
// declare exactly the same features. An invocation with no `--features` yields the
// empty set, which differs from any non-empty reference -- that is the exact
// regression this exists to catch.
const reference = invocations[0]
for (const inv of invocations.slice(1)) {
  const missing = [...reference.features].filter((f) => !inv.features.has(f))
  const extra = [...inv.features].filter((f) => !reference.features.has(f))
  if (missing.length || extra.length) {
    const parts = []
    if (missing.length) parts.push(`missing ${missing.join(', ')}`)
    if (extra.length) parts.push(`declares extra ${extra.join(', ')}`)
    failures.push(
      `${inv.where} does not match ${reference.where} (${parts.join('; ')}). ` +
        `Reference declares: ${[...reference.features].join(', ') || '(none)'}.`
    )
  }
}

if (failures.length > 0) {
  console.error('[verify-feature-parity] FAILED\n')
  for (const f of failures) console.error(`  - ${f}`)
  console.error(
    '\n  The local build must enable the same Cargo features the pipeline builds.\n' +
      '  A developer running `pnpm build && pnpm test` should exercise the artifact\n' +
      '  CI and release actually test.\n'
  )
  process.exit(1)
}

console.log(
  `[verify-feature-parity] OK — ${invocations.length} \`napi build\` invocation(s) ` +
    `across package.json and ${workflows.length} workflow(s) all declare: ` +
    `${[...reference.features].join(', ') || '(none)'}`
)