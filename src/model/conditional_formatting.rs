//! Conditional formatting: worksheet `<conditionalFormatting>` rules per OOXML §18.3.1.
//!
//! A conditional format attaches one or more `CfRule`s to a cell range (`sqref`).
//! Rule types (`cellIs`, `expression`, `colorScale`, `dataBar`, `iconSet`, `top10`,
//! `unique`, `duplicate`, `containsText`, `timePeriod`, blanks/errors/nonBlanks)
//! mirror the ExcelJS `ws.addConditionalFormatting` API. Rules carrying a `style`
//! reference a differential format (`dxf`) in `xl/styles.xml` via `dxfId`;
//! `colorScale` / `dataBar` / `iconSet` carry their visuals inline and use no `dxfId`.

use napi_derive::napi;

/// A single color used by `colorScale` / `dataBar` / `iconSet` rules.
#[napi(object)]
#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct CfColor {
    /// ARGB hex (8 chars) or RGB hex (6 chars).
    pub argb: Option<String>,
    /// Theme color index (`theme="N"`).
    pub theme: Option<u32>,
    /// Indexed palette color (`indexed="N"`).
    pub indexed: Option<u32>,
    /// Tint applied to the color.
    pub tint: Option<f64>,
}

/// A conditional-format value object (`cfvo`): a stop on a `colorScale`,
/// `dataBar`, or `iconSet` rule.
#[napi(object)]
#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct Cfvo {
    /// Value type: `num` | `percent` | `percentile` | `formula` | `min` | `max`
    /// | `autoMin` | `autoMax`.
    pub r#type: String,
    /// Value (for `num` / `percent` / `percentile` / `formula`); absent for
    /// `min` / `max` / `autoMin` / `autoMax`.
    pub value: Option<String>,
}

/// One conditional-format rule.
#[napi(object)]
#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct CfRule {
    /// Rule type: `cellIs` | `expression` | `colorScale` | `dataBar` | `iconSet`
    /// | `top10` | `unique` | `duplicate` | `containsText` | `timePeriod`
    /// | `containsBlanks` | `notContainsBlanks` | `containsErrors` | `notContainsErrors`.
    pub r#type: String,
    /// Worksheet-global unique 1-based priority.
    pub priority: u32,
    /// Index into the workbook `dxfs` collection (rules with a `style` only).
    /// `None` for `colorScale` / `dataBar` / `iconSet`.
    pub dxf_id: Option<u32>,
    /// `cellIs` operator: `lessThan` | `greaterThan` | `equal` | `notEqual`
    /// | `greaterThanOrEqual` | `lessThanOrEqual` | `between` | `notBetween`.
    /// `containsText` operator: `containsText` | `beginsWith` | `endsWith`
    /// | `notContainsText`.
    pub operator: Option<String>,
    /// Formula(s): `cellIs` / `expression` / `containsText` use one or two.
    pub formula: Option<Vec<String>>,
    /// `containsText` text (the substring/pattern to match).
    pub text: Option<String>,
    /// `timePeriod` value: `today` | `yesterday` | `tomorrow` | `last7Days`
    /// | `lastWeek` | `thisWeek` | `nextWeek` | `lastMonth` | `thisMonth` | `nextMonth`.
    pub time_period: Option<String>,
    /// `top10` rank (count of top/bottom items or percent).
    pub rank: Option<u32>,
    /// `top10` percent flag.
    pub percent: Option<bool>,
    /// `top10` bottom flag (select bottom instead of top).
    pub bottom: Option<bool>,
    /// Differential style applied to matching cells (all rule types except
    /// `colorScale` / `dataBar` / `iconSet`).
    pub style: Option<crate::model::style::Style>,
    /// `colorScale` / `dataBar` / `iconSet` value objects (stops).
    pub cfvo: Option<Vec<Cfvo>>,
    /// `colorScale` colors (one per `cfvo`).
    pub color: Option<Vec<CfColor>>,
    /// `dataBar` color (single).
    pub data_bar_color: Option<CfColor>,
    /// `iconSet` name (e.g. `3TrafficLights`).
    pub icon_set: Option<String>,
    /// `iconSet` reverse flag.
    pub reverse: Option<bool>,
    /// `dataBar` / `iconSet` show-value flag.
    pub show_value: Option<bool>,
}

/// A conditional format: a cell range plus its rules.
#[napi(object)]
#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct ConditionalFormat {
    /// Cell range reference (e.g. `"A1:A10"` or `"A1:A10 C1:C10"`).
    pub sqref: String,
    /// Rules applied to `sqref`.
    pub rules: Vec<CfRule>,
}
