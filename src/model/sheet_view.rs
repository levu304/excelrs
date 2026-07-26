//! Worksheet view state: freeze/split pane descriptors per OOXML CT_SheetView.
//!
//! Each `<sheetView>` may carry a `<pane>` child with split/freeze dimensions.
//! excelrs exposes these as an array (matching ExcelJS `worksheet.views`).

use napi_derive::napi;

/// Sheet view pane state.
#[napi(string_enum)]
#[derive(Clone, Debug, Default, PartialEq)]
pub enum SheetViewState {
    #[default]
    Frozen,
    Split,
}

impl std::fmt::Display for SheetViewState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SheetViewState::Frozen => write!(f, "frozen"),
            SheetViewState::Split => write!(f, "split"),
        }
    }
}

impl From<&str> for SheetViewState {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "split" => SheetViewState::Split,
            _ => SheetViewState::Frozen,
        }
    }
}

/// Active pane quadrant.
#[napi(string_enum)]
#[derive(Clone, Debug, Default, PartialEq)]
pub enum ActivePane {
    #[default]
    BottomLeft,
    BottomRight,
    TopLeft,
    TopRight,
}

impl std::fmt::Display for ActivePane {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ActivePane::BottomLeft => write!(f, "bottomLeft"),
            ActivePane::BottomRight => write!(f, "bottomRight"),
            ActivePane::TopLeft => write!(f, "topLeft"),
            ActivePane::TopRight => write!(f, "topRight"),
        }
    }
}

impl From<&str> for ActivePane {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "bottomright" => ActivePane::BottomRight,
            "topleft" => ActivePane::TopLeft,
            "topright" => ActivePane::TopRight,
            _ => ActivePane::BottomLeft,
        }
    }
}

/// A single sheet view descriptor, mirroring a `<sheetView><pane>` pair.
#[napi(object)]
#[derive(Clone, Debug, Default)]
pub struct SheetView {
    /// Pane state: `"frozen"` | `"split"` | absent.
    pub state: Option<SheetViewState>,
    /// Horizontal split position (number of columns frozen/split).
    pub x_split: Option<u32>,
    /// Vertical split position (number of rows frozen/split).
    pub y_split: Option<u32>,
    /// The top-left visible cell in the bottom-right pane (e.g. "A1").
    pub top_left_cell: Option<String>,
    /// Active pane identifier: "bottomLeft", "bottomRight", "topLeft", "topRight".
    pub active_pane: Option<ActivePane>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sheet_view_default() {
        let sv = SheetView::default();
        assert!(sv.state.is_none());
        assert!(sv.x_split.is_none());
        assert!(sv.y_split.is_none());
    }

    #[test]
    fn test_sheet_view_frozen() {
        let sv = SheetView {
            state: Some("frozen".into()),
            x_split: Some(1),
            y_split: Some(2),
            top_left_cell: Some("B3".into()),
            active_pane: Some("bottomRight".into()),
        };
        assert_eq!(sv.state, Some(SheetViewState::Frozen));
        assert_eq!(sv.x_split, Some(1));
    }
}
