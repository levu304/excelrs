## 1. Add `mark_formula` builder

- [ ] 1.1 In `src/model/cell.rs` `impl CellValue`, add `pub fn mark_formula(mut self, formula: impl Into<String>) -> Self` that sets `self.value_type = "Formula"` and `self.formula = Some(formula.into())`, returning `self`.

## 2. Use the builder at the call site

- [ ] 2.1 In `src/model/worksheet.rs` `insert_cell_formula`, replace `cv.value_type = "Formula".to_string(); cv.formula = Some(formula);` with `cv = cv.mark_formula(formula);` (the binding is already `let mut cv`).
- [ ] 2.2 Remove the now-obsolete comment explaining why a raw assignment was necessary.

## 3. Verify behavior-neutral

- [ ] 3.1 `cargo test --features formula-eval` passes.
- [ ] 3.2 `napi build` (or `npm run build`) succeeds; call-site types unchanged.
- [ ] 3.3 Grep confirms no remaining `cv.value_type = "Formula"` mutation outside `cell.rs` `mark_formula`.
