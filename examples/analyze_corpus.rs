//! Corpus analysis tool for the typework-formula-corpus spike.
//!
//! Reads every `.xlsx` under a directory (recursively), extracts all formula
//! strings from every sheet, and reports:
//!   - per-function frequency (absolute + % of formulas containing it)
//!   - the leading function of each formula (for the ranked table)
//!   - coverage: % of formulas the shipped excelrs engine can fully evaluate
//!     (every referenced function is in the shipped set) vs. those that would
//!     fail without the missing lookups
//!   - cross-sheet reference frequency (`Sheet!A1`)
//!
//! Usage:
//!   cargo run --example analyze_corpus -- <corpus-dir> [corpus-label]
//!
//! Output is JSON on stdout (captured by the report author) plus a short
//! human summary on stderr.

use std::collections::BTreeMap;
use std::io::BufReader;
use std::fs::File;
use std::path::{Path, PathBuf};

use calamine::{open_workbook, Reader, Xlsx};
use serde_json::json;

/// Functions the shipped engine dispatches in `src/formula/bridge.rs`.
/// `CONCATENATE` and `CONCAT` share one arm; TRUE/FALSE are parser literals,
/// not function-call names, so they are not in this set.
const SHIPPED_FUNCS: &[&str] = &[
    "SUM", "AVERAGE", "MIN", "MAX", "COUNT", "COUNTA", "IF", "AND", "OR", "NOT",
    "ABS", "ROUND", "CONCATENATE", "CONCAT", "LEFT", "RIGHT", "MID", "LEN", "IFERROR",
];

/// The lookup family named in issue #51 (the gap set).
const GAP_LOOKUPS: &[&str] = &["INDEX", "MATCH", "XLOOKUP", "VLOOKUP"];

/// Broader lookup/reference family also unsupported (surfaced for awareness).
const GAP_BROADER: &[&str] = &[
    "HLOOKUP", "LOOKUP", "CHOOSE", "OFFSET", "INDIRECT", "XMATCH", "FILTER", "SORT",
    "UNIQUE", "TRANSPOSE", "ARRAYTOTEXT", "CHOOSEROWS", "CHOOSECOLS",
];

fn is_shipped(f: &str) -> bool {
    SHIPPED_FUNCS.contains(&f)
}

fn find_xlsx(dir: &Path, out: &mut Vec<PathBuf>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                find_xlsx(&p, out);
            } else if p.extension().and_then(|x| x.to_str()) == Some("xlsx") {
                out.push(p);
            }
        }
    }
}

/// Extract `NAME(` function-call tokens (uppercase-starting identifiers).
fn function_tokens(formula: &str) -> Vec<String> {
    let mut toks = Vec::new();
    let bytes = formula.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        // identifier start
        if bytes[i].is_ascii_uppercase() || bytes[i] == b'_' {
            let start = i;
            i += 1;
            while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_' || bytes[i] == b'.') {
                i += 1;
            }
            // must be followed by '('
            let mut j = i;
            while j < bytes.len() && (bytes[j] == b' ' || bytes[j] == b'\t' || bytes[j] == b'\n') {
                j += 1;
            }
            if j < bytes.len() && bytes[j] == b'(' {
                toks.push(formula[start..i].to_uppercase());
            }
        } else {
            i += 1;
        }
    }
    toks
}

/// Leading function name, if the formula begins with `NAME(`.
fn leading_function(formula: &str) -> Option<String> {
    let s = formula.trim_start_matches('=').trim_start();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() && (bytes[i].is_ascii_uppercase() || bytes[i] == b'_') {
        i += 1;
    }
    if i == 0 {
        return None;
    }
    let name = &s[..i];
    let mut j = i;
    while j < bytes.len() && (bytes[j] == b' ' || bytes[j] == b'\t') {
        j += 1;
    }
    if j < bytes.len() && bytes[j] == b'(' {
        Some(name.to_uppercase())
    } else {
        None
    }
}

#[derive(serde::Serialize)]
struct WorkbookStats {
    file: String,
    formulas: usize,
    cross_sheet: usize,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let dir = args.get(1).cloned().unwrap_or_else(|| ".".to_string());
    let label = args.get(2).cloned().unwrap_or_else(|| dir.clone());

    let mut files = Vec::new();
    find_xlsx(Path::new(&dir), &mut files);
    files.sort();
    eprintln!("corpus label: {label}");
    eprintln!("xlsx files found: {}", files.len());

    let mut total_formulas: usize = 0;
    let mut cross_sheet: usize = 0;
    let mut fully_evaluable: usize = 0;
    let mut needs_lookup: usize = 0;
    let mut needs_broader: usize = 0;

    // function -> count of formulas containing it
    let mut fn_count: BTreeMap<String, usize> = BTreeMap::new();
    // leading function -> count
    let mut leading_count: BTreeMap<String, usize> = BTreeMap::new();
    let mut workbooks: Vec<WorkbookStats> = Vec::new();

    for f in &files {
        let mut wb = match open_workbook::<Xlsx<BufReader<File>>, _>(f) {
            Ok(w) => w,
            Err(e) => {
                eprintln!("skip {}: {e}", f.display());
                continue;
            }
        };
        let names = wb.sheet_names().to_vec();
        let mut wb_formulas = 0usize;
        let mut wb_cross = 0usize;
        for name in &names {
            let range = match wb.worksheet_formula(name) {
                Ok(r) => r,
                Err(_) => continue,
            };
            for (_r, _c, cell) in range.cells() {
                let formula = cell.trim();
                if formula.is_empty() {
                    continue;
                }
                wb_formulas += 1;
                total_formulas += 1;

                let xref = formula.contains('!');
                if xref {
                    wb_cross += 1;
                    cross_sheet += 1;
                }

                let toks = function_tokens(formula);
                let mut unsupported = false;
                let mut has_lookup = false;
                let mut has_broader = false;
                for t in &toks {
                    *fn_count.entry(t.clone()).or_insert(0) += 1;
                    if !is_shipped(t) {
                        unsupported = true;
                        if GAP_LOOKUPS.contains(&t.as_str()) {
                            has_lookup = true;
                        }
                        if GAP_BROADER.contains(&t.as_str()) {
                            has_broader = true;
                        }
                    }
                }
                if !unsupported {
                    fully_evaluable += 1;
                }
                if has_lookup {
                    needs_lookup += 1;
                }
                if has_broader {
                    needs_broader += 1;
                }

                if let Some(lead) = leading_function(formula) {
                    *leading_count.entry(lead).or_insert(0) += 1;
                }
            }
        }
        workbooks.push(WorkbookStats {
            file: f.file_name().and_then(|x| x.to_str()).unwrap_or("?").to_string(),
            formulas: wb_formulas,
            cross_sheet: wb_cross,
        });
    }

    let denom = total_formulas.max(1);
    let pct = |n: usize| (n as f64 / denom as f64) * 100.0;

    // All functions seen, ranked by frequency (full picture incl. shipped).
    let mut all_fns: Vec<(String, usize)> = fn_count.iter().map(|(k, v)| (k.clone(), *v)).collect();
    all_fns.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));

    let report = json!({
        "corpusLabel": label,
        "fileCount": files.len(),
        "totalFormulas": total_formulas,
        "crossSheetFormulas": cross_sheet,
        "crossSheetPct": pct(cross_sheet),
        "fullyEvaluableByShipped": fully_evaluable,
        "fullyEvaluablePct": pct(fully_evaluable),
        "needsMissingLookup": needs_lookup,
        "needsMissingLookupPct": pct(needs_lookup),
        "needsBroaderUnsupported": needs_broader,
        "needsBroaderUnsupportedPct": pct(needs_broader),
        "functionFrequency": all_fns,
        "leadingFunctionFrequency": leading_count,
        "workbooks": workbooks,
    });

    println!("{}", serde_json::to_string_pretty(&report).unwrap());
}