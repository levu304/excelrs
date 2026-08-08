// KIS rich-text repro: write one rich-text cell, save output.xlsx.
// Goal (task 1.1): confirm current inline-string output renders Calibri in
// Apple Numbers (broken baseline). Goal (task 6.1): confirm shared-string
// output renders per-run fonts (fix).
const { Workbook } = require('../index')

async function main() {
  const wb = new Workbook()
  const ws = wb.addWorksheet('S')
  ws.getCell('A1').value = {
    richText: [
      { text: 'B: (11) = (7) + (10)\n', font: { name: 'Times New Roman' } },
      { text: 'bold tail', font: { name: 'Times New Roman', bold: true } },
    ],
  }
  await wb.xlsx.writeFile('output.xlsx')
  console.log('wrote output.xlsx')
}

main().catch((e) => {
  console.error(e)
  process.exit(1)
})