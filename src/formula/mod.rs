//! Formula evaluation module.
//!
//! Gated behind the `formula-eval` Cargo feature, which pulls in
//! `xlstream-parse` (wraps the surviving `formulizer-parse` parser) and
//! `xlstream-core` (value types: `Value`, `CellError`, `ExcelDate`).
//!
//! The evaluator in [`bridge`] walks the xlstream-parse AST, resolves cell
//! and range references through the excelrs data model, applies operators
//! with sticky error propagation, and dispatches ~20 built-in functions.

#[cfg(feature = "formula-eval")]
mod bridge;

#[cfg(feature = "formula-eval")]
pub use bridge::{evaluate_formula, FormulaEvaluator, Scalar, value_to_cell_value};

#[cfg(all(test, feature = "formula-eval"))]
mod tests;

