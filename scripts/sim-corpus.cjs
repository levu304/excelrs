// Simulate the typework corpus (#1 from the spike): generate a handful of
// representative business-report workbooks with realistic formulas, so the
// committed analyzer (examples/analyze_corpus.rs) can produce a directional
// go/no-go number WITHOUT typework's real files.
//
//   node scripts/sim-corpus.cjs            # writes ./corpus-sim/*.xlsx
//   cargo run --example analyze_corpus -- corpus-sim "SIM typework-style"
//
// This is SIMULATED data, not real typework workbooks. Use only to validate
// the pipeline and show what a representative run looks like.

const ExcelJS = require("exceljs");
const fs = require("fs");
const path = require("path");

const OUT = path.join(__dirname, "..", "corpus-sim");
fs.mkdirSync(OUT, { recursive: true });

// helper: build a workbook with sheets of {address, value|formula}
async function make(name, sheets) {
  const wb = new ExcelJS.Workbook();
  wb.creator = "sim-corpus";
  for (const [title, cells] of sheets) {
    const ws = wb.addWorksheet(title);
    for (const c of cells) {
      const cell = ws.getCell(c.a);
      if (c.f !== undefined) cell.value = { formula: c.f };
      else cell.value = c.v;
    }
  }
  const p = path.join(OUT, name);
  await wb.xlsx.writeFile(p);
  console.log("wrote", p);
}

async function main() {
  // 1) Sales report — detail + summary, cross-sheet, some VLOOKUP
  await make("sales-report.xlsx", [
    [
      "Orders",
      [
        { a: "A1", v: "Region" }, { a: "B1", v: "Units" }, { a: "C1", v: "Price" },
        { a: "A2", v: "West" }, { a: "B2", v: 10 }, { a: "C2", v: 5 },
        { a: "A3", v: "East" }, { a: "B3", v: 8 }, { a: "C3", v: 6 },
        { a: "A4", v: "West" }, { a: "B4", v: 12 }, { a: "C4", v: 5 },
        { a: "A5", v: "North" }, { a: "B5", v: 7 }, { a: "C5", v: 7 },
        { a: "A6", v: "East" }, { a: "B6", v: 9 }, { a: "C6", v: 6 },
      ],
    ],
    [
      "Summary",
      [
        { a: "A1", v: "Total units" },
        { a: "B1", f: "SUM(Orders!B2:B6)" },
        { a: "A2", v: "Avg price" },
        { a: "B2", f: "AVERAGE(Orders!C2:C6)" },
        { a: "A3", v: "West units" },
        { a: "B3", f: "SUMIF(Orders!A2:A6,\"West\",Orders!B2:B6)" },
        { a: "A4", v: "Big orders" },
        { a: "B4", f: "COUNTIF(Orders!B2:B6,\">10\")" },
        { a: "A5", v: "Discount flag" },
        { a: "B5", f: "IF(B1>40,\"bulk\",\"retail\")" },
        { a: "A6", v: "Lookup price" },
        { a: "B6", f: "VLOOKUP(\"East\",Orders!A2:C6,3,FALSE)" },
      ],
    ],
  ]);

  // 2) Inventory — thresholds, flags, counts
  await make("inventory.xlsx", [
    [
      "Stock",
      [
        { a: "A1", v: "Item" }, { a: "B1", v: "OnHand" }, { a: "C1", v: "Reorder" },
        { a: "A2", v: "A" }, { a: "B2", v: 3 }, { a: "C2", v: 10 },
        { a: "A3", v: "B" }, { a: "B3", v: 25 }, { a: "C3", v: 8 },
        { a: "A4", v: "C" }, { a: "B4", v: 0 }, { a: "C4", v: 5 },
        { a: "D1", v: "Total" }, { a: "D2", f: "SUM(B2:B4)" },
        { a: "E1", v: "Min" }, { a: "E2", f: "MIN(B2:B4)" },
        { a: "F1", v: "Max" }, { a: "F2", f: "MAX(B2:B4)" },
        { a: "G1", v: "Need reorder" }, { a: "G2", f: "COUNTIF(B2:B4,\"<5\")" },
        { a: "H1", v: "Low flag" }, { a: "H2", f: "IF(B2<C2,\"LOW\",\"ok\")" },
        { a: "H3", f: "IF(B3<C3,\"LOW\",\"ok\")" },
        { a: "H4", f: "IF(B4<C4,\"LOW\",\"ok\")" },
      ],
    ],
  ]);

  // 3) Finance dashboard — summary referencing detail, text/round
  await make("finance-dashboard.xlsx", [
    [
      "Ledger",
      [
        { a: "A1", v: "Q1" }, { a: "B1", v: 1000 },
        { a: "A2", v: "Q2" }, { a: "B2", v: 1500 },
        { a: "A3", v: "Q3" }, { a: "B3", v: 1300 },
        { a: "A4", v: "Q4" }, { a: "B4", v: 1700 },
      ],
    ],
    [
      "Dashboard",
      [
        { a: "A1", v: "Revenue" }, { a: "B1", f: "SUM(Ledger!B1:B4)" },
        { a: "A2", v: "Avg quarter" }, { a: "B2", f: "ROUND(AVERAGE(Ledger!B1:B4),0)" },
        { a: "A3", v: "Best" }, { a: "B3", f: "MAX(Ledger!B1:B4)" },
        { a: "A4", v: "Label" }, { a: "B4", f: 'TEXT(B1,"$#,##0")' },
        { a: "A5", v: "Non-neg" }, { a: "B5", f: "IF(B1>0,B1,0)" },
        { a: "A6", v: "Counted" }, { a: "B6", f: "COUNTA(Ledger!A1:A4)" },
      ],
    ],
  ]);

  // 4) Project tracker — status logic, completion %
  await make("project-tracker.xlsx", [
    [
      "Tasks",
      [
        { a: "A1", v: "Task" }, { a: "B1", v: "Done" }, { a: "C1", v: "Hours" },
        { a: "A2", v: "Design" }, { a: "B2", v: 1 }, { a: "C2", v: 20 },
        { a: "A3", v: "Build" }, { a: "B3", v: 1 }, { a: "C3", v: 80 },
        { a: "A4", v: "Test" }, { a: "B4", v: 0 }, { a: "C4", v: 30 },
        { a: "D1", v: "Done count" }, { a: "D2", f: "COUNTIF(B2:B4,1)" },
        { a: "E1", v: "Total hours" }, { a: "E2", f: "SUM(C2:C4)" },
        { a: "F1", v: "Status" }, { a: "F2", f: 'IF(D2=3,"complete","in progress")' },
        { a: "G1", v: "Pct" }, { a: "G2", f: "ROUND(D2/3,2)" },
      ],
    ],
  ]);

  // 5) KPI summary — SUMIFS / AVERAGEIF, cross-sheet
  await make("kpi-summary.xlsx", [
    [
      "Metrics",
      [
        { a: "A1", v: "Team" }, { a: "B1", v: "Score" },
        { a: "A2", v: "A" }, { a: "B2", v: 90 },
        { a: "A3", v: "B" }, { a: "B3", v: 70 },
        { a: "A4", v: "A" }, { a: "B4", v: 85 },
        { a: "C1", v: "Team A total" }, { a: "C2", f: 'SUMIFS(B2:B4,A2:A4,"A")' },
        { a: "D1", v: "Team A avg" }, { a: "D2", f: 'AVERAGEIF(A2:A4,"A",B2:B4)' },
        { a: "E1", v: "High" }, { a: "E2", f: "COUNTIF(B2:B4,\">80\")" },
      ],
    ],
  ]);

  // 6) Data entry — mostly values, light formulas
  await make("data-entry.xlsx", [
    [
      "Input",
      [
        { a: "A1", v: 5 }, { a: "A2", v: 10 }, { a: "A3", v: 15 },
        { a: "B1", v: "Sum" }, { a: "B2", f: "SUM(A1:A3)" },
        { a: "C1", v: "Double" }, { a: "C2", f: "A1*2" },
        { a: "D1", v: "Note" }, { a: "D2", f: 'IF(A1>0,"ok","no")' },
      ],
    ],
  ]);

  console.log("simulated corpus written to", OUT);
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});