//! Custom formula evaluator built on [`xlstream_parse`].
//!
//! `xlstream-parse` wraps the surviving `formularizer-parse` crate for AST
//! parsing. The evaluation engine itself is implemented here — walking the
//! AST, resolving cell/range references through excelrs's model, applying
//! operators with sticky error propagation, and dispatching a subset of
//! built-in functions (SUM, AVERAGE, MIN, MAX, COUNT, IF, etc.).

use std::collections::HashSet;

use xlstream_core::{CellError, ExcelDate, Value};
use xlstream_parse::{parse, NodeRef, NodeView};

use crate::error::ExcelrsError;
use crate::model::cell::{CellType, CellValue};
use crate::model::worksheet::Worksheet;
use crate::model::workbook_inner::WorkbookInner;

/// A scalar result value (re-export of xlstream-core's value type).
pub type Scalar = xlstream_core::Value;

/// Internal evaluation outcome — value, 2D array, or sticky error.
#[derive(Clone)]
enum Outcome {
    Value(Value),
    Array(Vec<Vec<Value>>),
    Error(CellError),
}

/// Cell reference key for cycle detection: (sheet_index, row, col).
type CellKey = u64;

fn cell_key(sheet_idx: usize, row: u32, col: u32) -> CellKey {
    (u64::try_from(sheet_idx).unwrap_or(0) << 32)
        | (u64::from(row) << 16)
        | u64::from(col)
}

fn cell_error_to_string(e: CellError) -> String {
    match e {
        CellError::Div0 => "#DIV/0!".into(),
        CellError::Value => "#VALUE!".into(),
        CellError::Ref => "#REF!".into(),
        CellError::Name => "#NAME?".into(),
        CellError::Na => "#N/A".into(),
        CellError::Num => "#NUM!".into(),
        CellError::Null => "#NULL!".into(),
    }
}

fn parse_error_string(s: &str) -> CellError {
    match s {
        "#DIV/0!" => CellError::Div0,
        "#VALUE!" => CellError::Value,
        "#REF!" => CellError::Ref,
        "#NAME?" => CellError::Name,
        "#N/A" => CellError::Na,
        "#NUM!" => CellError::Num,
        "#NULL!" => CellError::Null,
        _ => CellError::Value,
    }
}

/// Convert excelrs [`CellValue`] to [`Value`].
fn cell_value_to_value(cv: &CellValue) -> Value {
    match CellType::from_tag(&cv.value_type) {
        // "Integer" tag is never produced by CellValue constructors (calamine
        // Value::Integer maps to "Number" in value_to_cell_value); falls through to _.
        Some(CellType::Number) => cv.number.map_or(Value::Empty, Value::Number),
        Some(CellType::String) => cv
            .string
            .as_ref()
            .map(|s| Value::Text(s.as_str().into()))
            .unwrap_or(Value::Empty),
        Some(CellType::Boolean) => Value::Bool(cv.boolean.unwrap_or(false)),
        Some(CellType::Error) => {
            if let Some(ref e) = cv.error_value {
                Value::Error(parse_error_string(e))
            } else {
                Value::Empty
            }
        }
        Some(CellType::Date) => cv
            .date_serial
            .map(|s| Value::Date(ExcelDate { serial: s }))
            .unwrap_or(Value::Empty),
        Some(CellType::Formula) => {
            // Use cached value fields (set by recalculate)
            if let Some(n) = cv.number {
                return Value::Number(n);
            }
            if let Some(ref s) = cv.string {
                return Value::Text(s.as_str().into());
            }
            if let Some(b) = cv.boolean {
                return Value::Bool(b);
            }
            if let Some(ref e) = cv.error_value {
                return Value::Error(parse_error_string(e));
            }
            if let Some(serial) = cv.date_serial {
                return Value::Date(ExcelDate { serial });
            }
            Value::Empty
        }
        _ => Value::Empty,
    }
}

/// Convert [`Value`] to excelrs [`CellValue`] for caching.
pub fn value_to_cell_value(v: &Value) -> CellValue {
    match v {
        Value::Empty => CellValue::default(),
        Value::Number(n) => CellValue::number(*n),
        Value::Integer(i) => CellValue::number(*i as f64),
        Value::Text(s) => CellValue::string(s.to_string()),
        Value::Bool(b) => CellValue::boolean(*b),
        // `CellValue::date` sets type "Date" without the `number` field; the
        // original code uses "Number" with both `number` and `date_serial`.
        Value::Date(d) => CellValue {
            value_type: "Number".to_string(),
            number: Some(d.serial),
            date_serial: Some(d.serial),
            ..Default::default()
        },
        // No `CellValue` constructor for Error variant — set field directly.
        Value::Error(e) => CellValue {
            value_type: "Error".to_string(),
            error_value: Some(cell_error_to_string(*e)),
            ..Default::default()
        },
    }
}

/// Strip the leading `=` that Excel may store in formula text.
fn normalize_formula(formula: &str) -> &str {
    formula.strip_prefix('=').unwrap_or(formula)
}

/// Stateless evaluator that walks the xlstream-parse AST and resolves
/// cell/range references through the excelrs data model.
///
/// Created per evaluation — no persistent engine state stored on the model.
pub struct FormulaEvaluator<'ws> {
    worksheet: &'ws Worksheet,
    sheet_name: String,
    workbook: Option<&'ws WorkbookInner>,
    sheet_index_cache: Vec<(String, usize)>,
}

impl<'ws> FormulaEvaluator<'ws> {
    pub fn new(
        worksheet: &'ws Worksheet,
        sheet_name: String,
        workbook: Option<&'ws WorkbookInner>,
    ) -> Self {
        Self {
            worksheet,
            sheet_name,
            workbook,
            sheet_index_cache: Vec::new(),
        }
    }

    fn sheet_index(&mut self, name: &str) -> Option<usize> {
        if let Some(&(_, idx)) = self
            .sheet_index_cache
            .iter()
            .find(|(n, _)| n == name)
        {
            return Some(idx);
        }
        let wb = self.workbook?;
        let idx = wb.worksheets.iter().position(|ws| ws.name() == name);
        if let Some(i) = idx {
            self.sheet_index_cache.push((name.to_string(), i));
        }
        idx
    }

    fn resolve_worksheet(&mut self, sheet: Option<&str>) -> Option<Worksheet> {
        match sheet {
            None => Some(self.worksheet.clone()),
            Some(name) if name == self.sheet_name => Some(self.worksheet.clone()),
            Some(name) => {
                let wb = self.workbook?;
                let idx = self.sheet_index(name)?;
                Some(wb.worksheets[idx].clone())
            }
        }
    }

    /// Evaluate a formula string. `row`/`col` are the formula cell's position
    /// (1-based) for cycle detection in recursive references.
    pub fn evaluate(
        &mut self,
        formula: &str,
        row: u32,
        col: u32,
    ) -> Result<Option<Scalar>, ExcelrsError> {
        let norm = normalize_formula(formula);
        let ast = parse(norm)
            .map_err(|e| ExcelrsError::Parse(format!("formula parse error: {}", e)))?;
        let mut seen: HashSet<CellKey> = HashSet::new();
        seen.insert(cell_key(0, row, col));
        let outcome = self.eval_node(ast.root(), &mut seen);
        Ok(Some(Self::outcome_to_value(outcome)))
    }

    fn eval_node(&mut self, node: NodeRef, seen: &mut HashSet<CellKey>) -> Outcome {
        match node.view() {
            NodeView::Number(n) => Outcome::Value(Value::Number(n)),
            NodeView::Bool(b) => Outcome::Value(Value::Bool(b)),
            NodeView::Text(s) => Outcome::Value(Value::Text(s.into())),
            NodeView::Error(e) => Outcome::Error(e),

            NodeView::CellRef { sheet, row, col } => {
                self.eval_cell_ref(sheet, row, col, seen)
            }

            NodeView::RangeRef {
                sheet,
                start_row,
                start_col,
                end_row,
                end_col,
            } => match self.eval_range_ref(
                sheet, start_row, start_col, end_row, end_col, seen,
            ) {
                Ok(grid) => Outcome::Array(grid),
                Err(e) => Outcome::Error(e),
            },

            NodeView::NamedRef(_) => Outcome::Error(CellError::Name),
            NodeView::ExternalRef { .. } => Outcome::Error(CellError::Ref),
            NodeView::TableRef { .. } => Outcome::Error(CellError::Name),
            NodeView::ThreeDimensionalRef { .. } => Outcome::Error(CellError::Ref),
            NodeView::PreludeRef(_) => Outcome::Error(CellError::Name),

            NodeView::BinaryOp { op } => {
                let left = node.left();
                let right = node.right();
                match (left, right) {
                    (Some(l), Some(r)) => {
                        let lv = self.eval_node(l, seen);
                        let rv = self.eval_node(r, seen);
                        Self::apply_binary_op(op, lv, rv)
                    }
                    _ => Outcome::Error(CellError::Value),
                }
            }

            NodeView::UnaryOp { op } => {
                let operand = node.operand();
                match operand {
                    Some(o) => {
                        let ov = self.eval_node(o, seen);
                        Self::apply_unary_op(op, ov)
                    }
                    None => Outcome::Error(CellError::Value),
                }
            }

            NodeView::Function { name } => {
                let args: Vec<Outcome> = node
                    .args()
                    .iter()
                    .map(|a| self.eval_node(*a, seen))
                    .collect();
                Self::call_function(name, args)
            }

            NodeView::Array { .. } => {
                let cells = node.array_cells();
                let grid: Vec<Vec<Value>> = cells
                    .iter()
                    .map(|row| {
                        row.iter()
                            .map(|c| {
                                let o = self.eval_node(*c, seen);
                                match o {
                                    Outcome::Value(v) => v,
                                    Outcome::Error(e) => Value::Error(e),
                                    Outcome::Array(g) => g
                                        .first()
                                        .and_then(|r| r.first())
                                        .cloned()
                                        .unwrap_or(Value::Empty),
                                }
                            })
                            .collect()
                    })
                    .collect();
                Outcome::Array(grid)
            }
        }
    }

    fn eval_cell_ref(
        &mut self,
        sheet: Option<&str>,
        row: u32,
        col: u32,
        seen: &mut HashSet<CellKey>,
    ) -> Outcome {
        let ws = match self.resolve_worksheet(sheet) {
            Some(ws) => ws,
            None => return Outcome::Error(CellError::Ref),
        };

        let ws_name = ws.name();
        let sidx = if ws_name == self.sheet_name {
            0
        } else {
            self.sheet_index(&ws_name).unwrap_or(0)
        };
        let key = cell_key(sidx, row, col);

        if seen.contains(&key) {
            return Outcome::Error(CellError::Ref);
        }
        seen.insert(key);

        let cell = ws.get_cell_by_rc(row, col);
        let cv = cell.value_raw();

        let outcome = if matches!(CellType::from_tag(&cv.value_type), Some(CellType::Formula)) {
            if let Some(ref formula) = cv.formula {
                let norm = normalize_formula(formula);
                match parse(norm) {
                    Ok(ast) => {
                        let root = ast.root();
                        self.eval_node(root, seen)
                    }
                    Err(_) => Outcome::Error(CellError::Value),
                }
            } else {
                Outcome::Value(Value::Empty)
            }
        } else {
            Outcome::Value(cell_value_to_value(&cv))
        };

        seen.remove(&key);
        outcome
    }

    fn eval_range_ref(
        &mut self,
        sheet: Option<&str>,
        start_row: Option<u32>,
        start_col: Option<u32>,
        end_row: Option<u32>,
        end_col: Option<u32>,
        seen: &mut HashSet<CellKey>,
    ) -> Result<Vec<Vec<Value>>, CellError> {
        let ws = self.resolve_worksheet(sheet).ok_or(CellError::Ref)?;

        let sr = start_row.unwrap_or(1);
        let sc = start_col.unwrap_or(1);
        let er = end_row.unwrap_or_else(|| ws.row_count());
        let ec = end_col.unwrap_or_else(|| ws.column_count());

        let mut grid: Vec<Vec<Value>> = Vec::new();
        for r in sr..=er {
            let mut row_vals: Vec<Value> = Vec::new();
            for c in sc..=ec {
                let outcome = self.eval_cell_ref(sheet, r, c, seen);
                row_vals.push(Self::outcome_to_value(outcome));
            }
            grid.push(row_vals);
        }
        Ok(grid)
    }

    // --- Outcome helpers ---

    fn outcome_to_value(outcome: Outcome) -> Value {
        match outcome {
            Outcome::Value(v) => v,
            Outcome::Error(e) => Value::Error(e),
            Outcome::Array(grid) => grid
                .first()
                .and_then(|row| row.first())
                .cloned()
                .unwrap_or(Value::Empty),
        }
    }

    /// Flatten argument outcomes, unpacking arrays into individual values.
    fn flatten_args(args: &[Outcome]) -> Result<Vec<Value>, CellError> {
        let mut result = Vec::new();
        for arg in args {
            match arg {
                Outcome::Value(v) => result.push(v.clone()),
                Outcome::Array(grid) => {
                    for row in grid {
                        for cell_v in row {
                            result.push(cell_v.clone());
                        }
                    }
                }
                Outcome::Error(e) => return Err(*e),
            }
        }
        Ok(result)
    }

    /// Extract numeric values from argument outcomes for math functions.
    fn collect_numbers(args: &[Outcome]) -> Result<Vec<f64>, CellError> {
        let mut nums = Vec::new();
        for arg in args {
            match arg {
                Outcome::Value(Value::Number(n)) => nums.push(*n),
                Outcome::Value(Value::Integer(i)) => nums.push(*i as f64),
                Outcome::Value(Value::Bool(b)) => nums.push(if *b { 1.0 } else { 0.0 }),
                Outcome::Value(Value::Date(d)) => nums.push(d.serial),
                Outcome::Value(Value::Text(s)) => {
                    if let Ok(n) = s.parse::<f64>() {
                        nums.push(n);
                    }
                }
                Outcome::Value(Value::Error(e)) => return Err(*e),
                Outcome::Array(grid) => {
                    for row in grid {
                        for cell_v in row {
                            match cell_v {
                                Value::Number(n) => nums.push(*n),
                                Value::Integer(i) => nums.push(*i as f64),
                                Value::Bool(b) => nums.push(if *b { 1.0 } else { 0.0 }),
                                Value::Date(d) => nums.push(d.serial),
                                _ => {}
                            }
                        }
                    }
                }
                Outcome::Error(e) => return Err(*e),
                _ => {}
            }
        }
        Ok(nums)
    }

    fn as_f64(v: &Value) -> Result<f64, CellError> {
        match v {
            Value::Number(n) => Ok(*n),
            Value::Integer(i) => Ok(*i as f64),
            Value::Bool(b) => Ok(if *b { 1.0 } else { 0.0 }),
            Value::Date(d) => Ok(d.serial),
            Value::Text(s) => s.parse::<f64>().map_err(|_| CellError::Value),
            Value::Error(e) => Err(*e),
            _ => Err(CellError::Value),
        }
    }

    fn is_truthy(v: &Value) -> bool {
        match v {
            Value::Number(n) => *n != 0.0,
            Value::Integer(i) => *i != 0,
            Value::Bool(b) => *b,
            Value::Text(s) => s.parse::<f64>().map(|n| n != 0.0).unwrap_or(false),
            Value::Date(d) => d.serial != 0.0,
            _ => false,
        }
    }

    fn to_display_string(v: &Value) -> Result<String, CellError> {
        match v {
            Value::Text(s) => Ok(s.to_string()),
            Value::Number(n) => Ok(Self::fmt_num(*n)),
            Value::Integer(i) => Ok(i.to_string()),
            Value::Bool(b) => Ok(if *b { "TRUE" } else { "FALSE" }.to_string()),
            Value::Empty => Ok(String::new()),
            Value::Error(e) => Err(*e),
            _ => Err(CellError::Value),
        }
    }

    fn fmt_num(n: f64) -> String {
        if n.fract() == 0.0 && n.abs() < 1e15 {
            format!("{}", n as i64)
        } else {
            format!("{}", n)
        }
    }

    // --- Operator dispatch ---

    fn apply_binary_op(op: &str, left: Outcome, right: Outcome) -> Outcome {
        // Error short-circuit (borrow by ref to avoid move)
        if let Outcome::Error(e) = &left {
            return Outcome::Error(*e);
        }
        if let Outcome::Error(e) = &right {
            return Outcome::Error(*e);
        }
        // Extract Values, converting arrays to first-element (implicit intersection)
        let (l, r) = match (&left, &right) {
            (Outcome::Value(l), Outcome::Value(r)) => (l.clone(), r.clone()),
            _ => (Self::outcome_to_value(left), Self::outcome_to_value(right)),
        };
        match op {
            "+" => Self::arith_add(l, r),
            "-" => Self::arith_sub(l, r),
            "*" => Self::arith_mul(l, r),
            "/" => Self::arith_div(l, r),
            "^" => Self::arith_pow(l, r),
            "%" => Self::arith_mod(l, r),
            "=" => Outcome::Value(Self::cmp_val(l, r, |a, b| a == b)),
            "<>" => Outcome::Value(Self::cmp_val(l, r, |a, b| a != b)),
            "<" => Outcome::Value(Self::cmp_val(l, r, |a, b| a < b)),
            ">" => Outcome::Value(Self::cmp_val(l, r, |a, b| a > b)),
            "<=" => Outcome::Value(Self::cmp_val(l, r, |a, b| a <= b)),
            ">=" => Outcome::Value(Self::cmp_val(l, r, |a, b| a >= b)),
            "&" => Self::arith_concat(l, r),
            _ => Outcome::Error(CellError::Value),
        }
    }

    fn apply_unary_op(op: &str, operand: Outcome) -> Outcome {
        if let Outcome::Error(e) = &operand {
            return Outcome::Error(*e);
        }
        let v = Self::outcome_to_value(operand);
        match op {
            "-" => Self::arith_neg(v),
            "%" => Outcome::Value(Self::percent(v)),
            _ => Outcome::Error(CellError::Value),
        }
    }

    // --- Arithmetic ---

    fn arith_add(l: Value, r: Value) -> Outcome {
        Self::arith(l, r, |a, b| a + b)
    }
    fn arith_sub(l: Value, r: Value) -> Outcome {
        Self::arith(l, r, |a, b| a - b)
    }
    fn arith_mul(l: Value, r: Value) -> Outcome {
        Self::arith(l, r, |a, b| a * b)
    }
    fn arith_div(l: Value, r: Value) -> Outcome {
        match (Self::as_f64(&l), Self::as_f64(&r)) {
            (Ok(_), Ok(0.0)) => Outcome::Error(CellError::Div0),
            (Ok(a), Ok(b)) => Outcome::Value(Value::Number(a / b)),
            (Err(e), _) | (_, Err(e)) => Outcome::Error(e),
        }
    }
    fn arith_pow(l: Value, r: Value) -> Outcome {
        Self::arith(l, r, |a, b| a.powf(b))
    }
    fn arith_mod(l: Value, r: Value) -> Outcome {
        match (Self::as_f64(&l), Self::as_f64(&r)) {
            (Ok(_), Ok(0.0)) => Outcome::Error(CellError::Div0),
            (Ok(a), Ok(b)) => Outcome::Value(Value::Number(a % b)),
            (Err(e), _) | (_, Err(e)) => Outcome::Error(e),
        }
    }

    fn arith(l: Value, r: Value, op: impl Fn(f64, f64) -> f64) -> Outcome {
        match (Self::as_f64(&l), Self::as_f64(&r)) {
            (Ok(a), Ok(b)) => Outcome::Value(Value::Number(op(a, b))),
            (Err(e), _) | (_, Err(e)) => Outcome::Error(e),
        }
    }

    fn arith_concat(l: Value, r: Value) -> Outcome {
        match (Self::to_display_string(&l), Self::to_display_string(&r)) {
            (Ok(a), Ok(b)) => {
                let mut s = String::with_capacity(a.len() + b.len());
                s.push_str(&a);
                s.push_str(&b);
                Outcome::Value(Value::Text(s.into()))
            }
            (Err(e), _) | (_, Err(e)) => Outcome::Error(e),
        }
    }

    fn arith_neg(v: Value) -> Outcome {
        match Self::as_f64(&v) {
            Ok(n) => Outcome::Value(Value::Number(-n)),
            Err(e) => Outcome::Error(e),
        }
    }

    fn percent(v: Value) -> Value {
        match v {
            Value::Number(n) => Value::Number(n / 100.0),
            Value::Integer(i) => Value::Number(i as f64 / 100.0),
            _ => Value::Error(CellError::Value),
        }
    }

    fn cmp_val(l: Value, r: Value, op: impl Fn(f64, f64) -> bool) -> Value {
        match (Self::as_f64(&l), Self::as_f64(&r)) {
            (Ok(a), Ok(b)) => Value::Bool(op(a, b)),
            (Err(e), _) | (_, Err(e)) => Value::Error(e),
        }
    }

    // --- Built-in functions ---

    fn call_function(name: &str, args: Vec<Outcome>) -> Outcome {
        match name.to_uppercase().as_str() {
            "SUM" => Self::fn_sum(args),
            "AVERAGE" => Self::fn_average(args),
            "MIN" => Self::fn_min_max(args, true),
            "MAX" => Self::fn_min_max(args, false),
            "COUNT" => Self::fn_count(args),
            "COUNTA" => Self::fn_counta(args),
            "IF" => Self::fn_if(args),
            "AND" => Self::fn_and(args),
            "OR" => Self::fn_or(args),
            "NOT" => Self::fn_not(args),
            "ABS" => Self::fn_abs(args),
            "ROUND" => Self::fn_round(args),
            "CONCATENATE" | "CONCAT" => Self::fn_concat(args),
            "LEFT" => Self::fn_left(args),
            "RIGHT" => Self::fn_right(args),
            "MID" => Self::fn_mid(args),
            "LEN" => Self::fn_len(args),
            "IFERROR" => Self::fn_iferror(args),
            _ => Outcome::Error(CellError::Name),
        }
    }

    fn fn_sum(args: Vec<Outcome>) -> Outcome {
        match Self::collect_numbers(&args) {
            Ok(nums) => Outcome::Value(Value::Number(nums.iter().sum())),
            Err(e) => Outcome::Error(e),
        }
    }

    fn fn_average(args: Vec<Outcome>) -> Outcome {
        match Self::collect_numbers(&args) {
            Ok(nums) if nums.is_empty() => Outcome::Error(CellError::Div0),
            Ok(nums) => {
                let sum: f64 = nums.iter().sum();
                Outcome::Value(Value::Number(sum / nums.len() as f64))
            }
            Err(e) => Outcome::Error(e),
        }
    }

    fn fn_min_max(args: Vec<Outcome>, is_min: bool) -> Outcome {
        match Self::collect_numbers(&args) {
            Ok(nums) if nums.is_empty() => Outcome::Value(Value::Number(0.0)),
            Ok(nums) => {
                let result = if is_min {
                    nums.iter().copied().fold(f64::INFINITY, f64::min)
                } else {
                    nums.iter().copied().fold(f64::NEG_INFINITY, f64::max)
                };
                Outcome::Value(Value::Number(result))
            }
            Err(e) => Outcome::Error(e),
        }
    }

    fn fn_count(args: Vec<Outcome>) -> Outcome {
        match Self::flatten_args(&args) {
            Ok(vals) => {
                let count = vals
                    .iter()
                    .filter(|v| matches!(v, Value::Number(_) | Value::Integer(_)))
                    .count();
                Outcome::Value(Value::Integer(count as i64))
            }
            Err(e) => Outcome::Error(e),
        }
    }

    fn fn_counta(args: Vec<Outcome>) -> Outcome {
        match Self::flatten_args(&args) {
            Ok(vals) => {
                let count = vals
                    .iter()
                    .filter(|v| !matches!(v, Value::Empty))
                    .count();
                Outcome::Value(Value::Integer(count as i64))
            }
            Err(e) => Outcome::Error(e),
        }
    }

    fn fn_if(args: Vec<Outcome>) -> Outcome {
        if args.is_empty() {
            return Outcome::Error(CellError::Value);
        }
        match &args[0] {
            Outcome::Error(e) => Outcome::Error(*e),
            Outcome::Value(v) if Self::is_truthy(v) => {
                if args.len() > 1 {
                    args[1].clone()
                } else {
                    Outcome::Value(Value::Empty)
                }
            }
            _ => {
                if args.len() > 2 {
                    args[2].clone()
                } else {
                    Outcome::Value(Value::Empty)
                }
            }
        }
    }

    fn fn_and(args: Vec<Outcome>) -> Outcome {
        match Self::flatten_args(&args) {
            Ok(vals) => {
                let result = vals.iter().all(Self::is_truthy);
                Outcome::Value(Value::Bool(result))
            }
            Err(e) => Outcome::Error(e),
        }
    }

    fn fn_or(args: Vec<Outcome>) -> Outcome {
        match Self::flatten_args(&args) {
            Ok(vals) => {
                let result = vals.iter().any(Self::is_truthy);
                Outcome::Value(Value::Bool(result))
            }
            Err(e) => Outcome::Error(e),
        }
    }

    fn fn_not(args: Vec<Outcome>) -> Outcome {
        if args.len() != 1 {
            return Outcome::Error(CellError::Value);
        }
        match &args[0] {
            Outcome::Error(e) => Outcome::Error(*e),
            Outcome::Value(v) => Outcome::Value(Value::Bool(!Self::is_truthy(v))),
            Outcome::Array(_) => {
                let v = Self::outcome_to_value(args[0].clone());
                Outcome::Value(Value::Bool(!Self::is_truthy(&v)))
            }
        }
    }

    fn fn_abs(args: Vec<Outcome>) -> Outcome {
        if args.len() != 1 {
            return Outcome::Error(CellError::Value);
        }
        match &args[0] {
            Outcome::Error(e) => Outcome::Error(*e),
            Outcome::Value(Value::Number(n)) => Outcome::Value(Value::Number(n.abs())),
            Outcome::Value(Value::Integer(i)) => Outcome::Value(Value::Integer(i.abs())),
            _ => Outcome::Error(CellError::Value),
        }
    }

    fn fn_round(args: Vec<Outcome>) -> Outcome {
        if args.is_empty() || args.len() > 2 {
            return Outcome::Error(CellError::Value);
        }
        let val = match &args[0] {
            Outcome::Error(e) => return Outcome::Error(*e),
            Outcome::Value(v) => match Self::as_f64(v) {
                Ok(n) => n,
                Err(e) => return Outcome::Error(e),
            },
            _ => return Outcome::Error(CellError::Value),
        };
        let digits = if args.len() == 2 {
            match &args[1] {
                Outcome::Value(v) => match Self::as_f64(v) {
                    Ok(n) => n.round() as i32,
                    Err(e) => return Outcome::Error(e),
                },
                Outcome::Error(e) => return Outcome::Error(*e),
                _ => return Outcome::Error(CellError::Value),
            }
        } else {
            0
        };
        let factor = 10f64.powi(digits);
        Outcome::Value(Value::Number((val * factor).round() / factor))
    }

    fn fn_concat(args: Vec<Outcome>) -> Outcome {
        match Self::flatten_args(&args) {
            Ok(vals) => {
                let mut result = String::new();
                for v in &vals {
                    match Self::to_display_string(v) {
                        Ok(s) => result.push_str(&s),
                        Err(e) => return Outcome::Error(e),
                    }
                }
                Outcome::Value(Value::Text(result.into()))
            }
            Err(e) => Outcome::Error(e),
        }
    }

    fn fn_left(args: Vec<Outcome>) -> Outcome {
        Self::fn_left_right(args, true)
    }

    fn fn_right(args: Vec<Outcome>) -> Outcome {
        Self::fn_left_right(args, false)
    }

    fn fn_left_right(args: Vec<Outcome>, from_left: bool) -> Outcome {
        if args.is_empty() || args.len() > 2 {
            return Outcome::Error(CellError::Value);
        }
        let text = match Self::to_display_string(&Self::outcome_to_value(args[0].clone())) {
            Ok(s) => s,
            Err(e) => return Outcome::Error(e),
        };
        let chars: Vec<char> = text.chars().collect();
        let len = if args.len() == 2 {
            match Self::as_f64(&Self::outcome_to_value(args[1].clone())) {
                Ok(n) => n.round() as usize,
                Err(e) => return Outcome::Error(e),
            }
        } else {
            1
        };
        if from_left {
            Outcome::Value(Value::Text(chars.iter().take(len).collect::<String>().into()))
        } else {
            let start = chars.len().saturating_sub(len);
            Outcome::Value(Value::Text(chars[start..].iter().collect::<String>().into()))
        }
    }

    fn fn_mid(args: Vec<Outcome>) -> Outcome {
        if args.len() != 3 {
            return Outcome::Error(CellError::Value);
        }
        let text = match Self::to_display_string(&Self::outcome_to_value(args[0].clone())) {
            Ok(s) => s,
            Err(e) => return Outcome::Error(e),
        };
        let start = match Self::as_f64(&Self::outcome_to_value(args[1].clone())) {
            Ok(n) => n.round() as usize,
            Err(e) => return Outcome::Error(e),
        };
        let len = match Self::as_f64(&Self::outcome_to_value(args[2].clone())) {
            Ok(n) => n.round() as usize,
            Err(e) => return Outcome::Error(e),
        };
        let chars: Vec<char> = text.chars().collect();
        let start_idx = start.saturating_sub(1);
        let end_idx = (start_idx + len).min(chars.len());
        if start_idx >= chars.len() {
            Outcome::Value(Value::Text(String::new().into()))
        } else {
            Outcome::Value(Value::Text(
                chars[start_idx..end_idx].iter().collect::<String>().into(),
            ))
        }
    }

    fn fn_len(args: Vec<Outcome>) -> Outcome {
        if args.len() != 1 {
            return Outcome::Error(CellError::Value);
        }
        let v = Self::outcome_to_value(args[0].clone());
        match &v {
            Value::Text(s) => Outcome::Value(Value::Integer(s.chars().count() as i64)),
            Value::Number(n) => {
                Outcome::Value(Value::Integer(Self::fmt_num(*n).chars().count() as i64))
            }
            Value::Integer(i) => {
                Outcome::Value(Value::Integer(i.to_string().chars().count() as i64))
            }
            Value::Bool(b) => {
                let s = if *b { "TRUE" } else { "FALSE" };
                Outcome::Value(Value::Integer(s.len() as i64))
            }
            Value::Error(e) => Outcome::Error(*e),
            _ => Outcome::Error(CellError::Value),
        }
    }

    fn fn_iferror(args: Vec<Outcome>) -> Outcome {
        if args.is_empty() || args.len() > 2 {
            return Outcome::Error(CellError::Value);
        }
        match &args[0] {
            Outcome::Error(_) | Outcome::Value(Value::Error(_)) => {
                if args.len() > 1 {
                    args[1].clone()
                } else {
                    Outcome::Value(Value::Empty)
                }
            }
            other => other.clone(),
        }
    }
}

/// Public convenience: evaluate a formula against a worksheet.
/// For cell-by-cell evaluation use `FormulaEvaluator::new` + `evaluate`.
pub fn evaluate_formula(
    formula: &str,
    worksheet: &Worksheet,
    sheet_name: &str,
    workbook: Option<&WorkbookInner>,
) -> Result<Option<Scalar>, ExcelrsError> {
    let mut evaluator = FormulaEvaluator::new(worksheet, sheet_name.to_string(), workbook);
    evaluator.evaluate(formula, 0, 0)
}