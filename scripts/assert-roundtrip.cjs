// Shared round-trip assertions for release verification.
//
// These three guarantees — cell style, merged ranges, row style — are what
// openspec/specs/release-verification requires the pipeline to prove before
// publication. Two call sites assert them against different load paths:
//
//   scripts/prepublish-smoke.cjs  loads via the hand-maintained index.js glue
//   scripts/musl-smoke-test.cjs   requires a .node path directly
//
// They were previously one assertion body inlined in each script. Sharing them
// means a new guarantee is added once and both in-process release paths assert it.
//
// Scope: this covers the two paths that run in-process against a binary. It does
// NOT cover the post-publish registry check in release.yml, which asserts
// packaging (that the published set resolves and loads) rather than behavior —
// behavioral guarantees there would contradict the requirement that assertions
// complete before publication.
//
// Kept dependency-free and side-effect-free apart from its own logging: these run
// inside release jobs where an unexpected require or an early process.exit would
// be indistinguishable from a product failure.

'use strict'

// Asserts that style, merges, and row-level style survive a write/read round-trip.
// `Workbook` is injected rather than required so the caller controls how the
// native binding is loaded. Throws on the first failed guarantee.
async function assertRoundTrip(Workbook, log = () => {}) {
  const wb = new Workbook()
  const ws = wb.addWorksheet('test')
  ws.addRow([1, 2, 3])
  ws.addRow(['x', 'y', 'z'])
  ws.addRow(['p', 'q', 'r'])

  // Cell-level style: font.bold + fill
  ws.setCellStyle(1, 2, {
    font: { bold: true },
    fill: { kind: 'Solid', foreground: 'FFFF0000' },
  })

  // Row-level style
  ws.getRow(3).style = {
    font: { bold: true, color: 'FFFF0000' },
    fill: { kind: 'Solid', foreground: 'FFFFFFFF' },
  }

  // Merge range
  ws.mergeCells('B2:D2')

  const buf = await wb.xlsx.write()
  if (buf.length < 100) {
    throw new Error('xlsx too small: ' + buf.length)
  }
  log('OK native binding works, xlsx size: ' + buf.length)

  const wb2 = new Workbook()
  await wb2.xlsx.read(buf)
  const ws2 = wb2.getWorksheet('test')

  // Cell style preserved
  const s = ws2.getCellByRc(1, 2).style
  if (!s || !s.font || s.font.bold !== true) {
    throw new Error('FAIL: font.bold not preserved on read-back')
  }
  if (!s.fill || s.fill.kind !== 'Solid' || s.fill.foreground !== 'FFFF0000') {
    throw new Error(
      'FAIL: fill not preserved on read-back (got ' +
        (s.fill && s.fill.kind) +
        '/' +
        (s.fill && s.fill.foreground) +
        ')',
    )
  }
  log('OK cell style (font.bold + fill) round-trips')

  // Merged range preserved
  const merges = ws2.mergedRanges || []
  if (!merges.includes('B2:D2')) {
    throw new Error('FAIL: merged range lost on read-back')
  }
  log('OK merged range round-trips')

  // Row style preserved
  const rs = ws2.getRow(3).style
  if (!rs || !rs.font || rs.font.bold !== true || rs.font.color !== 'FFFF0000') {
    throw new Error('FAIL: row font.bold/color not preserved on read-back')
  }
  if (!rs.fill || rs.fill.foreground !== 'FFFFFFFF') {
    throw new Error('FAIL: row fill not preserved on read-back')
  }
  log('OK row style round-trips')
}

module.exports = { assertRoundTrip }