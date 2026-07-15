## 1. Track A — Theme-color write (internal, no public API change)

- [x] 1.1 Add `pub struct Color { rgb: Option<String>, theme: Option<u8>, tint: Option<f64> }` in `src/model/color.rs`
- [x] 1.2 Update `reader/styles.rs::parse_color` to retain `theme` + `tint` alongside the resolved `rgb` (rgb stays for public display/back-compat)
- [x] 1.3 Replace the plain `Option<String>` color fields in `Font`/`Fill`/`Border` models with `Option<Color>`
- [x] 1.4 Update `writer/styles.rs` to emit the resolved ARGB (`<color rgb="..."/>`) for theme-origin colors, because downstream consumers (e.g. ExcelJS) cannot resolve `<color theme="N"/>` references back to a color
- [x] 1.5 Update the napi mapping so `Color` serializes to the resolved ARGB **string** (preserves the "color is a plain ARGB string" public contract)
- [x] 1.6 Add a round-trip test (F5 in theme-color.test.ts): themed file read→write→read preserves the resolved ARGB string in `cell.style.font.color` (ExcelJS re-read resolves the color)
- [x] 1.7 Update `ROADMAP.md` (v0.13.0 row: theme-color write → shipped) and `README.md` limitations

## 2. Track B — JS Date preservation (core napi type-bridge)

- [x] 2.1 Add `date_serial: Option<f64>` (`#[napi(skip)]`) field to `CellValue` in `src/model/cell.rs` (no chrono dep — Excel serial is stored as plain f64)
- [x] 2.2 Add `CellValue::date(serial: f64) -> Self` constructor + `value_type: "Date"` default
- [x] 2.3 Implement serial↔millis helpers (`serial_to_millis`, `millis_to_serial`) using `EXCEL_EPOCH_SERIAL` (25569.0 = 1970-01-01)
- [x] 2.4 Implement `is_date_format(fmt) -> bool` heuristic + `date_format_for_serial(serial) -> String` (date-only vs datetime format)
- [x] 2.5 Reader (`reader/xlsx.rs`): calamine `Data::DateTime(dt)` → `CellValue::date(dt.as_f64())` (was ISO-8601 string in `map_data`)
- [x] 2.6 Writer (`writer/xlsx.rs`): Date cell → serial in `<v>`, inject date numFmt via `date_format_for_serial` when cell/col has no format
- [x] 2.7 napi bridge: `value()` returns `napi::Either<CellValue, napi::JsDate>` (transmute to `'static`); `set_value` takes `napi::bindgen_prelude::Unknown` (auto-detect JsDate via `from_napi_value`, fall back to serde_json)
- [x] 2.8 Enable `napi5` feature in Cargo.toml for `JsDate`/`create_date` support
- [x] 2.9 Update `native.d.ts` (get → `CellValue | Date`) and `index.d.ts` (get/set, discriminant doc)
- [x] 2.10 Unit tests: `test_cell_value_date`, `test_serial_epoch_round_trip`, `test_is_date_format_heuristic`, `test_date_format_for_serial`
- [x] 2.11 Rust callers of `set_value(serde_json::json!(...))` changed to `set_value_raw(CellValue{...})` (setter now raw napi-only)
- [x] 2.12 Update `ROADMAP.md` (Date/DateTime → shipped v0.13.0) + `README.md` (Date limitation note)

**Semver note**: The Date read behavior change (was ISO-8601 string, now JS `Date`) is an intentional minor-boundary change. Consumers reading Date cells will now see `Date` objects instead of strings.
