//! Worksheet view state: freeze/split pane descriptors per OOXML CT_SheetView.
//!
//! Each `<sheetView>` may carry a `<pane>` child with split/freeze dimensions.
//! excelrs exposes these as an array (matching ExcelJS `worksheet.views`).

use napi_derive::napi;

/// A single sheet view descriptor, mirroring a `<sheetView><pane>` pair.
#[napi(object)]
#[derive(Clone, Debug)]
#[derive(Default)]
pub struct SheetView {
    /// Pane state: "frozen", "split", or absent (`""`).
    pub state: Option<String>,
    /// Horizontal split position (number of columns frozen/split).
    pub x_split: Option<u32>,
    /// Vertical split position (number of rows frozen/split).
    pub y_split: Option<u32>,
    /// The top-left visible cell in the bottom-right pane (e.g. "A1").
    pub top_left_cell: Option<String>,
    /// Active pane identifier: "bottomLeft", "bottomRight", "topLeft", "topRight".
    pub active_pane: Option<String>,
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
        assert_eq!(sv.state.as_deref(), Some("frozen"));
        assert_eq!(sv.x_split, Some(1));
    }
}
