#![cfg(test)]

use crate::model::cell::CellValue;
use crate::model::worksheet::Worksheet;
use crate::formula::{FormulaEvaluator, evaluate_formula};
use xlstream_core::Value;

fn ws_with_numbers() -> Worksheet {
    let ws = Worksheet::new("Sheet1".into());
    ws.insert_cell_value(1, 1, CellValue::number(1.0));
    ws.insert_cell_value(1, 2, CellValue::number(2.0));
    ws.insert_cell_value(1, 3, CellValue::number(3.0));
    ws.insert_cell_value(2, 1, CellValue::number(4.0));
    ws.insert_cell_value(2, 2, CellValue::number(5.0));
    ws.insert_cell_value(2, 3, CellValue::number(6.0));
    ws
}

fn ws_with_text() -> Worksheet {
    let ws = Worksheet::new("Sheet1".into());
    ws.insert_cell_value(1, 1, CellValue::number(10.0));
    ws.insert_cell_value(1, 2, CellValue::string("text"));
    ws.insert_cell_value(1, 3, CellValue::number(20.0));
    ws
}

// === 5.1 Arithmetic ===

#[test]
fn test_arith_precedence() {
    let ws = Worksheet::new("Sheet1".into());
    let mut ev = FormulaEvaluator::new(&ws, "Sheet1".into(), None);
    let result = ev.evaluate("=1+2*3", 0, 0).unwrap().unwrap();
    assert_eq!(result, Value::Number(7.0));

    let result = ev.evaluate("=(1+2)*3", 0, 0).unwrap().unwrap();
    assert_eq!(result, Value::Number(9.0));
}

#[test]
fn test_arith_div_zero_error() {
    let ws = Worksheet::new("Sheet1".into());
    let mut ev = FormulaEvaluator::new(&ws, "Sheet1".into(), None);
    let result = ev.evaluate("=1/0", 0, 0).unwrap().unwrap();
    match result {
        Value::Error(e) => assert_eq!(format!("{:?}", e), "Div0"),
        other => panic!("expected Div0 error, got {:?}", other),
    }
}

#[test]
fn test_arith_error_short_circuit() {
    let ws = Worksheet::new("Sheet1".into());
    let mut ev = FormulaEvaluator::new(&ws, "Sheet1".into(), None);
    let result = ev.evaluate("=1/0+5", 0, 0).unwrap().unwrap();
    match result {
        Value::Error(e) => assert_eq!(format!("{:?}", e), "Div0"),
        other => panic!("expected Div0 error, got {:?}", other),
    }
}

#[test]
fn test_arith_all_operators() {
    let ws = Worksheet::new("Sheet1".into());
    let mut ev = FormulaEvaluator::new(&ws, "Sheet1".into(), None);

    assert_eq!(ev.evaluate("=10-4", 0, 0).unwrap().unwrap(), Value::Number(6.0));
    assert_eq!(ev.evaluate("=3*4", 0, 0).unwrap().unwrap(), Value::Number(12.0));
    assert_eq!(ev.evaluate("=10/2", 0, 0).unwrap().unwrap(), Value::Number(5.0));
    assert_eq!(ev.evaluate("=2^3", 0, 0).unwrap().unwrap(), Value::Number(8.0));
    assert_eq!(
        ev.evaluate("=\"hello\" & \" \" & \"world\"", 0, 0).unwrap().unwrap(),
        Value::Text("hello world".into())
    );
}

// === 5.2 Cell refs + cycle detection ===

#[test]
fn test_cell_ref_resolves_value() {
    let ws = ws_with_numbers();
    let mut ev = FormulaEvaluator::new(&ws, "Sheet1".into(), None);
    let result = ev.evaluate("=A1", 0, 0).unwrap().unwrap();
    assert_eq!(result, Value::Number(1.0));
}

#[test]
fn test_cell_ref_in_expression() {
    let ws = ws_with_numbers();
    let mut ev = FormulaEvaluator::new(&ws, "Sheet1".into(), None);
    let result = ev.evaluate("=A1+B1+C1", 0, 0).unwrap().unwrap();
    assert_eq!(result, Value::Number(6.0));
}

#[test]
fn test_circular_self_reference() {
    let ws = Worksheet::new("Sheet1".into());
    ws.insert_cell_formula(1, 1, "A1".to_string());
    let mut ev = FormulaEvaluator::new(&ws, "Sheet1".into(), None);
    let result = ev.evaluate("=A1", 1, 1).unwrap().unwrap();
    match result {
        Value::Error(e) => assert_eq!(format!("{:?}", e), "Ref"),
        other => panic!("expected Ref error, got {:?}", other),
    }
}

#[test]
fn test_circular_mutual_reference() {
    let ws = Worksheet::new("Sheet1".into());
    ws.insert_cell_formula(1, 1, "B1".to_string()); // A1 = "=B1"
    ws.insert_cell_formula(1, 2, "A1".to_string()); // B1 = "=A1"
    let mut ev = FormulaEvaluator::new(&ws, "Sheet1".into(), None);
    let result = ev.evaluate("=B1", 1, 1).unwrap().unwrap();
    match result {
        Value::Error(e) => assert_eq!(format!("{:?}", e), "Ref"),
        other => panic!("expected Ref error, got {:?}", other),
    }
}

#[test]
fn test_sibling_branch_not_cycle() {
    let ws = Worksheet::new("Sheet1".into());
    ws.insert_cell_value(1, 1, CellValue::number(10.0));
    ws.insert_cell_value(2, 1, CellValue::number(20.0));
    let mut ev = FormulaEvaluator::new(&ws, "Sheet1".into(), None);
    let result = ev.evaluate("=A1+A2", 0, 0).unwrap().unwrap();
    assert_eq!(result, Value::Number(30.0));
}

// === 5.3 Cross-sheet references ===

#[test]
fn test_cross_sheet_reference() {
    use crate::model::workbook_inner::WorkbookInner;

    let mut wb = WorkbookInner::new();
    let ws1 = Worksheet::new("Sheet1".into());
    ws1.insert_cell_value(1, 1, CellValue::number(42.0));
    let ws2 = Worksheet::new("Sheet2".into());
    wb.worksheets.push(ws1);
    wb.worksheets.push(ws2);

    let ws2_ref = &wb.worksheets[1];
    let mut ev = FormulaEvaluator::new(ws2_ref, "Sheet2".into(), Some(&wb));
    let result = ev.evaluate("='Sheet1'!A1", 0, 0).unwrap().unwrap();
    assert_eq!(result, Value::Number(42.0));
}

// === 5.4 Range refs ===

#[test]
fn test_range_ref_sum() {
    let ws = ws_with_numbers();
    let mut ev = FormulaEvaluator::new(&ws, "Sheet1".into(), None);
    let result = ev.evaluate("=SUM(A1:B2)", 0, 0).unwrap().unwrap();
    assert_eq!(result, Value::Number(12.0)); // 1+2+4+5
}

#[test]
fn test_range_ref_average() {
    let ws = ws_with_numbers();
    let mut ev = FormulaEvaluator::new(&ws, "Sheet1".into(), None);
    let result = ev.evaluate("=AVERAGE(A1:B2)", 0, 0).unwrap().unwrap();
    assert_eq!(result, Value::Number(3.0)); // (1+2+4+5)/4
}

#[test]
fn test_range_ref_min_max() {
    let ws = ws_with_numbers();
    let mut ev = FormulaEvaluator::new(&ws, "Sheet1".into(), None);
    assert_eq!(ev.evaluate("=MIN(A1:B2)", 0, 0).unwrap().unwrap(), Value::Number(1.0));
    assert_eq!(ev.evaluate("=MAX(A1:B2)", 0, 0).unwrap().unwrap(), Value::Number(5.0));
}

#[test]
fn test_whole_column_ref() {
    let ws = ws_with_numbers();
    ws.insert_cell_value(3, 1, CellValue::number(10.0)); // A3
    ws.insert_cell_value(4, 1, CellValue::number(20.0)); // A4
    let mut ev = FormulaEvaluator::new(&ws, "Sheet1".into(), None);
    let result = ev.evaluate("=SUM(A:A)", 0, 0).unwrap().unwrap();
    assert_eq!(result, Value::Number(35.0)); // A1+A2+A3+A4 = 1+4+10+20
}

#[test]
fn test_count_functions() {
    let ws = ws_with_text();
    let mut ev = FormulaEvaluator::new(&ws, "Sheet1".into(), None);
    assert_eq!(ev.evaluate("=COUNT(A1:C1)", 0, 0).unwrap().unwrap(), Value::Integer(2));
    assert_eq!(ev.evaluate("=COUNTA(A1:C1)", 0, 0).unwrap().unwrap(), Value::Integer(3));
}

#[test]
fn test_count_functions_range() {
    let ws = ws_with_numbers();
    let mut ev = FormulaEvaluator::new(&ws, "Sheet1".into(), None);
    assert_eq!(ev.evaluate("=COUNT(A1:B2)", 0, 0).unwrap().unwrap(), Value::Integer(4));
}

// === 5.5 Error sentinel propagation ===

#[test]
fn test_error_sentinel_from_ast() {
    let ws = Worksheet::new("Sheet1".into());
    let mut ev = FormulaEvaluator::new(&ws, "Sheet1".into(), None);
    let result = ev.evaluate("=#DIV/0!", 0, 0).unwrap().unwrap();
    match result {
        Value::Error(e) => assert_eq!(format!("{:?}", e), "Div0"),
        other => panic!("expected Div0 error, got {:?}", other),
    }
}

#[test]
fn test_if_function() {
    let ws = ws_with_numbers();
    let mut ev = FormulaEvaluator::new(&ws, "Sheet1".into(), None);
    let result = ev.evaluate("=IF(A1>0,\"yes\",\"no\")", 0, 0).unwrap().unwrap();
    assert_eq!(result, Value::Text("yes".into()));
}

#[test]
fn test_and_or_not() {
    let ws = Worksheet::new("Sheet1".into());
    let mut ev = FormulaEvaluator::new(&ws, "Sheet1".into(), None);
    assert_eq!(ev.evaluate("=AND(TRUE,FALSE)", 0, 0).unwrap().unwrap(), Value::Bool(false));
    assert_eq!(ev.evaluate("=OR(TRUE,FALSE)", 0, 0).unwrap().unwrap(), Value::Bool(true));
    assert_eq!(ev.evaluate("=NOT(TRUE)", 0, 0).unwrap().unwrap(), Value::Bool(false));
}

#[test]
fn test_string_functions() {
    let ws = Worksheet::new("Sheet1".into());
    let mut ev = FormulaEvaluator::new(&ws, "Sheet1".into(), None);

    assert_eq!(
        ev.evaluate("=CONCAT(\"Hello\",\" \",\"World\")", 0, 0).unwrap().unwrap(),
        Value::Text("Hello World".into())
    );
    assert_eq!(ev.evaluate("=LEFT(\"Hello\",2)", 0, 0).unwrap().unwrap(), Value::Text("He".into()));
    assert_eq!(ev.evaluate("=RIGHT(\"Hello\",2)", 0, 0).unwrap().unwrap(), Value::Text("lo".into()));
    assert_eq!(ev.evaluate("=MID(\"Hello\",2,3)", 0, 0).unwrap().unwrap(), Value::Text("ell".into()));
    assert_eq!(ev.evaluate("=LEN(\"Hello\")", 0, 0).unwrap().unwrap(), Value::Integer(5));
    assert_eq!(ev.evaluate("=ABS(-42)", 0, 0).unwrap().unwrap(), Value::Number(42.0));
    assert_eq!(ev.evaluate("=ROUND(3.14159,2)", 0, 0).unwrap().unwrap(), Value::Number(3.14));
}

#[test]
fn test_iferror() {
    let ws = Worksheet::new("Sheet1".into());
    let mut ev = FormulaEvaluator::new(&ws, "Sheet1".into(), None);
    let result = ev.evaluate("=IFERROR(1/0,\"caught\")", 0, 0).unwrap().unwrap();
    assert_eq!(result, Value::Text("caught".into()));
}

#[test]
fn test_unsupported_function() {
    let ws = Worksheet::new("Sheet1".into());
    let mut ev = FormulaEvaluator::new(&ws, "Sheet1".into(), None);
    let result = ev.evaluate("=VLOOKUP(1,A1,2)", 0, 0).unwrap().unwrap();
    match result {
        Value::Error(e) => assert_eq!(format!("{:?}", e), "Name"),
        other => panic!("expected Name error, got {:?}", other),
    }
}

// === 5.6 Cached value + recalculation ===

#[test]
fn test_worksheet_recalculate_caches_values() {
    let ws = ws_with_numbers();
    ws.insert_cell_formula(3, 1, "A1+B1".to_string()); // C1 = A1+B1 = 3

    let result = ws.recalculate();
    assert!(result.is_ok());

    let cell = ws.get_cell_by_rc(3, 1);
    let cached = cell.cached_value();
    assert!(cached.is_some());
    let cv = cached.unwrap();
    assert_eq!(cv.value_type, "Number");
    assert_eq!(cv.number, Some(3.0));
}

#[test]
fn test_cached_value_null_without_recalculate() {
    let ws = ws_with_numbers();
    ws.insert_cell_formula(3, 1, "A1+B1".to_string());
    let cell = ws.get_cell_by_rc(3, 1);
    assert!(cell.cached_value().is_none());
}

#[test]
fn test_recalculate_error_caching() {
    let ws = Worksheet::new("Sheet1".into());
    ws.insert_cell_formula(1, 1, "1/0".to_string());
    ws.recalculate().unwrap();

    let cell = ws.get_cell_by_rc(1, 1);
    let cached = cell.cached_value();
    assert!(cached.is_some());
    assert_eq!(cached.unwrap().value_type, "Error");
}

// === 5.7 Public API ===

#[test]
fn test_evaluate_formula_public_api() {
    let ws = ws_with_numbers();
    let result = evaluate_formula("=A1*2", &ws, "Sheet1", None);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().unwrap(), Value::Number(2.0));
}

#[test]
fn test_recalculate_chained_formulas() {
    let ws = ws_with_numbers();
    ws.insert_cell_formula(3, 1, "A1+B1".to_string());     // A3 = 3
    ws.insert_cell_formula(3, 2, "A3*2".to_string());      // B3 = A3*2 = 6

    ws.recalculate().unwrap();

    let a3 = ws.get_cell_by_rc(3, 1);
    assert_eq!(a3.cached_value().unwrap().number, Some(3.0));

    let b3 = ws.get_cell_by_rc(3, 2);
    assert_eq!(b3.cached_value().unwrap().number, Some(6.0));
}