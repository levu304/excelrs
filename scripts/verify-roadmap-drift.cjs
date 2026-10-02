#!/usr/bin/env node
// scripts/verify-roadmap-drift.cjs
// Guards two invariants on ROADMAP.md that cost real accuracy when they broke:
//
//   1. Every version a status-bearing row names must exist as a released CHANGELOG.md entry.
//      Catches typos and versions that never shipped.
//   2. Every status value must be one the document's own Status legend defines.
//      Catches `targeted` rows pointing at releases that already shipped.
//
// This deliberately does NOT restate the matrix's contents. Which feature area maps to which
// version is authored by hand in ROADMAP.md; restating that list here is exactly how this
// corpus rotted. What is machine-checkable is only the consistency between the two
// documents, so that is all this checks.
//
// ponytail: ceiling — this verifies that a named version *exists* and that a status is in the
// legend. It cannot verify that a given version shipped a given feature, so flipping
// `unreleased` to `shipped | v2.9.1` on the formula-evaluation row passes even though v2.9.1
// did not publish it by default. Catching that needs a per-feature->version oracle, which
// means restating the matrix in code — the thing that rotted. Upgrade path if it bites: have
// the release workflow append each release's feature names to CHANGELOG.md in a parseable
// block, then match a row's version against that block instead of against version headers.
//
// Scope: the vocabulary check runs only on the Parity Matrix, identified as the status-bearing
// table that also declares a "Shipped In" column. The Prioritized Roadmap's per-release
// summary tables have a Status column too, but those are release notes rather than a status
// contract — their Status cells mix a decorated `✅ shipped` with free prose like
// "deferred to v0.13.0 — separate FFI type-bridging effort". Holding those to the legend
// would be enforcing a contract those tables never claimed.
//
// Version checking is likewise scoped to the matrix, but the `targeted`-for-a-shipped-release
// check below is document-wide, so the Prioritized Roadmap rows stay covered.

const fs = require('fs')

const ROADMAP = 'ROADMAP.md'
const CHANGELOG = 'CHANGELOG.md'

const failures = []

const roadmap = fs.existsSync(ROADMAP) ? fs.readFileSync(ROADMAP, 'utf8') : null
const changelog = fs.existsSync(CHANGELOG) ? fs.readFileSync(CHANGELOG, 'utf8') : null

if (roadmap === null) failures.push(`${ROADMAP} does not exist.`)
if (changelog === null) failures.push(`${CHANGELOG} does not exist.`)

if (failures.length > 0) {
  console.error('[verify-roadmap-drift] FAILED\n')
  for (const f of failures) console.error(`  - ${f}`)
  process.exit(1)
}

// --- Released versions, read from the changelog rather than hardcoded --------------------
// Aborted releases still parse as headers, so an aborted version counts as having been cut.
// That is intentional: a status cell naming an aborted release is not drift.
const released = new Set()
for (const m of changelog.matchAll(/^## \[(\d+\.\d+\.\d+)\]/gm)) released.add(m[1])

// --- Status vocabulary, read from the document's own legend -----------------------------
const legendStatuses = new Set()
const legend = roadmap.match(/^\*\*Status legend:\*\*$[\s\S]*?(?=\n---|\n## )/m)
if (legend === null) {
  failures.push(`${ROADMAP} has no "**Status legend:**" block; cannot check status vocabulary.`)
} else {
  for (const m of legend[0].matchAll(/^\- \*\*([^*]+)\*\*/gm)) legendStatuses.add(m[1].trim())
}
if (legendStatuses.size === 0) failures.push(`${ROADMAP} legend defines no statuses.`)

// Statuses that appear in rows but are not separately legend entries. `planned (distant)`
// is used in the matrix for the two deferred subsystems; it is a qualified `planned`, and
// requiring the legend to enumerate every qualifier would make the legend a second list to
// maintain.
const QUALIFIED = /^planned \(.+\)$/

const cells = (line) => line.split('|').slice(1, -1).map((c) => c.trim())

if (failures.length === 0) {
  const lines = roadmap.split('\n')

  for (let i = 0; i < lines.length; i++) {
    // A table's header row is a pipe row immediately followed by a | --- | separator.
    if (!lines[i].startsWith('|')) continue
    if (!/^\|[\s|:-]+\|?\s*$/.test(lines[i + 1] ?? '')) continue

    const header = cells(lines[i])
    const statusIdx = header.findIndex((h) => h.toLowerCase() === 'status')
    const versionIdx = header.findIndex((h) => h.toLowerCase() === 'shipped in')

    // Only the Parity Matrix declares statuses against a version column. See the scope note
    // at the top of this file.
    if (statusIdx === -1 || versionIdx === -1) continue

    for (let j = i + 2; j < lines.length; j++) {
      const line = lines[j]
      if (!line.startsWith('|')) break // table ended

      const row = cells(line)
      const status = row[statusIdx]
      const label = (row[0] ?? 'row').replace(/\*\*/g, '')

      // A section header row (`| **Worksheet structure** | | | |`) has an empty status.
      if (!status || status.length === 0) continue

      // Invariant 2: status must be in the document's own vocabulary.
      if (!legendStatuses.has(status) && !QUALIFIED.test(status)) {
        failures.push(
          `${ROADMAP}:${j + 1} "${label}" has status "${status}", which the Status ` +
            `legend does not define (known: ${[...legendStatuses].sort().join(', ')}).`
        )
        continue
      }

      // Invariant 1: a version named in a status-bearing row must have been released.
      // `unreleased` is the sanctioned "merged but not published" marker; an em dash is
      // the sanctioned "not versioned" marker.
      const version = (row[versionIdx] ?? '').replace(/[*]/g, '')
      if (version.length === 0 || version === 'unreleased' || version === '—') continue

      if (/^v\d+\.\d+\.\d+$/.test(version)) {
        const bare = version.slice(1)
        if (!released.has(bare)) {
          failures.push(
            `${ROADMAP}:${j + 1} "${label}" names ${version}, but CHANGELOG.md has no ` +
              `release for ${bare}.`
          )
        }
      }
    }
  }

  // Invariant 2, stated directly: no row may claim `targeted` for a release that already
  // shipped. This is the exact failure that let six rows sit stale, so it is checked
  // explicitly rather than left to the legend losing the word.
  for (const m of roadmap.matchAll(/\|\s*targeted\s*\|\s*v?(\d+\.\d+\.\d+)\s*\|/g)) {
    failures.push(
      `${ROADMAP} still marks a row \`targeted\` for v${m[1]}, which already shipped. ` +
        `Update the row to \`shipped\`, or back to \`planned\` if it is no longer committed.`
    )
  }
}

if (failures.length > 0) {
  console.error('[verify-roadmap-drift] FAILED\n')
  for (const f of failures) console.error(`  - ${f}`)
  console.error('')
  process.exit(1)
}

console.log(
  `[verify-roadmap-drift] OK — ${released.size} released versions known; every ` +
    `ROADMAP.md status and version agrees with the legend and CHANGELOG.md.`
)