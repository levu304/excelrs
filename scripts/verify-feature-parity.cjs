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
// Three ways a naive version of this check passes while the invariant is broken, all
// of which are handled here rather than waved off:
//
//   1. A whole source disappears. If release.yml is deleted, "every invocation that
//      exists agrees" is trivially true. REQUIRED_SOURCES is asserted to be present,
//      so deleting a source is a failure, not a pass.
//   2. Step boundaries are guessed from indentation. A re-indented step whose body
//      merges into a neighbouring `cargo test --features formula-eval` inherits that
//      feature set. Boundaries are matched at any indent, and, more importantly,
//      each `napi build` occurrence is compared on its OWN — never on a chunk.
//   3. Two builds share one step. `napi build --features a && napi build` would read
//      as "declares a". Each occurrence is compared separately.
//
// ponytail: ceiling — this matches tokens with regexes rather than parsing workflow
// YAML, so it has no YAML dependency. The trade-off is that the parsing is only as
// good as the probe suite that exercises it: each hole above was found by mutating
// a scratch copy of `.github/` and observing a wrong exit code, not by reading the
// code. Re-run those probes after touching the parsing here — a new invocation
// shape (a different flag order, a YAML anchor, a matrix-generated command) is
// exactly the case that can read as "no features declared".

const fs = require('fs')
const path = require('path')

const ROOT = path.resolve(__dirname, '..')
const PKG = path.join(ROOT, 'package.json')
const WORKFLOW_DIR = path.join(ROOT, '.github', 'workflows')

// The invariant is "local == CI == release". A source that is absent cannot agree,
// so its absence is a failure rather than a silently reduced comparison set.
const REQUIRED_SOURCES = [
  { label: 'package.json', file: PKG },
  { label: '.github/workflows/ci.yml', file: path.join(WORKFLOW_DIR, 'ci.yml') },
  { label: '.github/workflows/release.yml', file: path.join(WORKFLOW_DIR, 'release.yml') },
]

// Matches `--features a,b` / `--features a b` / `--features="a,b"`, capturing
// everything up to the next flag so a trailing `--js native.js` is not swallowed.
const FEATURES_RE = /--features[=\s]+([^\n]*)/g

// Any list item that could open a workflow step. Matched at one-or-more indent
// rather than a fixed 6 spaces, so re-indenting a step cannot silently merge its
// body into a neighbour's chunk.
const STEP_RE = /^([ \t]+)-\s+(?:name|uses|run|id|if|env|shell|with|working-directory):/gm

// A `napi build` command. `pnpm exec napi build` / `npx napi build` both match.
const BUILD_RE = /\bnapi[ \t]+build\b/g

// Parses one command's `--features` value. A YAML block scalar folds newlines into
// the command, and a trailing `\` continues it onto the next line; both are joined
// first so a continued value is not read as the literal feature `\`.
function parseFeatures(command) {
  const flattened = command.replace(/\\[ \t]*\r?\n[ \t]*/g, ' ')
  const found = new Set()
  for (const m of flattened.matchAll(FEATURES_RE)) {
    // Cut at the next flag, then split on whitespace and commas.
    const value = m[1].split(/\s--/)[0]
    for (const raw of value.split(/[\s,]+/)) {
      const feature = raw.replace(/^["']|["']$/g, '').trim()
      if (feature) found.add(feature)
    }
  }
  return found
}

// Returns every `napi build` occurrence in `text`, each paired with the feature set
// declared between this occurrence and the next one. Comparing occurrences (not
// chunks) is what stops a second, undeclared build in the same step from inheriting
// the first one's flags.
function findInvocations(text, where, onFound) {
  const starts = [...text.matchAll(BUILD_RE)]
  starts.forEach((m, i) => {
    const end = starts[i + 1] ? starts[i + 1].index : text.length
    // Start the command at the beginning of its own line, so a `pnpm`/`npx` prefix
    // and anything earlier on a folded line is included, and so a preceding
    // `cargo test --features` on the same physical line is excluded.
    const lineStart = text.lastIndexOf('\n', m.index) + 1
    const command = text.slice(lineStart, end)
    onFound({ where, features: parseFeatures(command) })
  })
  return starts.length
}

const invocations = []
const missingSources = []

// package.json: each script whose command actually invokes `napi build`.
const pkg = JSON.parse(fs.readFileSync(PKG, 'utf8'))
let pkgCount = 0
for (const [name, command] of Object.entries(pkg.scripts || {})) {
  if (!/\bnapi\s+build\b/.test(command)) continue
  pkgCount++
  findInvocations(command, `package.json script "${name}"`, (inv) =>
    invocations.push(inv),
  )
}
if (pkgCount === 0) missingSources.push('package.json (no script invokes `napi build`)')

// Workflows: every `napi build` occurrence, labelled with the step it sits in.
const workflows = fs.existsSync(WORKFLOW_DIR)
  ? fs.readdirSync(WORKFLOW_DIR).filter((f) => f.endsWith('.yml') || f.endsWith('.yaml'))
  : []
for (const file of workflows.sort()) {
  const filePath = path.join(WORKFLOW_DIR, file)
  const text = fs.readFileSync(filePath, 'utf8')

  // Slice per step so a failure can name the step. The label is the nearest
  // preceding `- name:`/`- uses:`; an unnamed step is labelled by line number so it
  // is still locatable.
  const bounds = [...text.matchAll(STEP_RE)]
  if (bounds.length === 0) continue
  bounds.forEach((m, i) => {
    const chunkStart = m.index
    const chunkEnd = bounds[i + 1] ? bounds[i + 1].index : text.length
    const chunk = text.slice(chunkStart, chunkEnd)
    if (!/\bnapi\s+build\b/.test(chunk)) return

    // STEP_RE stops at the colon, so read the label off the full line.
    const lineText = text.slice(chunkStart, text.indexOf('\n', chunkStart))
    const named = /-\s+(?:name|uses):\s*([^\n]*)/.exec(lineText)
    const line = text.slice(0, chunkStart).split('\n').length
    const label = named && named[1].trim()
      ? named[1].trim().replace(/^["']|["']$/g, '')
      : `<unnamed step at line ${line}>`
    findInvocations(chunk, `.github/workflows/${file} step "${label}"`, (inv) =>
      invocations.push(inv),
    )
  })
}

for (const { label, file } of REQUIRED_SOURCES) {
  if (!fs.existsSync(file)) {
    missingSources.push(`${label} (expected at ${path.relative(ROOT, file)})`)
  }
}

const failures = []

for (const m of missingSources) {
  failures.push(
    `${m} is required by the "local build matches the pipeline's feature set" ` +
      'invariant. A source that does not exist cannot agree with anything, so its ' +
      'absence is a failure, not a smaller comparison set.',
  )
}

if (invocations.length === 0) {
  console.error(
    '[verify-feature-parity] FAILED\n\n' +
      '  - No `napi build` invocation was found in package.json or .github/workflows/.\n' +
      '    This check has nothing to compare and would pass vacuously.\n',
  )
  process.exit(1)
}

// The reference set is the first local build script; every other invocation must
// declare exactly the same features. An invocation with no `--features` yields the
// empty set, which differs from any non-empty reference -- that is the exact
// regression this exists to catch.
const reference = invocations[0]

// Agreement on an EMPTY feature set is not parity, it is a degenerate comparison:
// every `napi build` in this repo declares at least one Cargo feature, so a state
// where all of them declare none means the flags were stripped wholesale and the
// comparison has nothing left to be true about. Fail rather than pass vacuously.
if (reference.features.size === 0) {
  failures.push(
    `${reference.where} declares no \`--features\`. Every \`napi build\` in this ` +
      'project declares at least one Cargo feature, so an all-empty comparison is a ' +
      'stripped flag set rather than a genuine agreement. Declare the intended ' +
      'features explicitly on every invocation.',
  )
}

for (const inv of invocations.slice(1)) {
  const missing = [...reference.features].filter((f) => !inv.features.has(f))
  const extra = [...inv.features].filter((f) => !reference.features.has(f))
  if (missing.length || extra.length) {
    const parts = []
    if (missing.length) parts.push(`missing ${missing.join(', ')}`)
    if (extra.length) parts.push(`declares extra ${extra.join(', ')}`)
    failures.push(
      `${inv.where} does not match ${reference.where} (${parts.join('; ')}). ` +
        `Reference declares: ${[...reference.features].join(', ') || '(none)'}.`,
    )
  }
}

if (failures.length > 0) {
  console.error('[verify-feature-parity] FAILED\n')
  for (const f of failures) console.error(`  - ${f}`)
  console.error(
    '\n  The local build must enable the same Cargo features the pipeline builds.\n' +
      '  A developer running `pnpm build && pnpm test` should exercise the artifact\n' +
      '  CI and release actually test.\n',
  )
  process.exit(1)
}

console.log(
  `[verify-feature-parity] OK — ${invocations.length} \`napi build\` invocation(s) ` +
    `across package.json and ${workflows.length} workflow(s) all declare: ` +
    `${[...reference.features].join(', ') || '(none)'}`
)
