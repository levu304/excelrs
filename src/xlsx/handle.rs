//! WorkbookXlsx — async XLSX I/O handle.
//!
//! Holds a shared reference (`Arc<Mutex<WorkbookInner>>`) to the parent
//! Workbook's state so that `read` / `readFile` can mutate the workbook
//! in place.  `write` / `writeFile` are stubs for v0.1.
//!
//! # Lock discipline
//! Read methods build the new `WorkbookInner` **outside** the lock
//! (calamine I/O can be slow) and then take the lock briefly to swap:
//! `*guard = new_inner`.  This avoids blocking other readers during I/O.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use napi::bindgen_prelude::*;
use napi_derive::napi;

use crate::model::workbook_inner::WorkbookInner;

/// Async XLSX read/write handle.
///
/// Obtained via `Workbook.xlsx` getter.  Shares the same underlying
/// `Arc<Mutex<WorkbookInner>>` as the parent Workbook.
#[napi]
#[derive(Clone, Debug)]
pub struct WorkbookXlsx {
    inner: Arc<Mutex<WorkbookInner>>,
}

impl WorkbookXlsx {
    pub(crate) fn new(inner: Arc<Mutex<WorkbookInner>>) -> Self {
        WorkbookXlsx { inner }
    }
}

#[napi]
impl WorkbookXlsx {
    /// Read an .xlsx file from a JS `Buffer`.  Async.
    ///
    /// Parses the buffer with calamine, then replaces the workbook state
    /// in-place.  All existing worksheets are discarded.
    ///
    /// @remarks Must be awaited.  The workbook state is only swapped when the
    /// returned Promise resolves.  Accessing worksheets before awaiting the
    /// Promise will see the old (stale) state.
    #[napi]
    pub async fn read(&self, buffer: Buffer) -> Result<()> {
        let data = buffer.to_vec();
        let new_inner = crate::reader::xlsx::workbook_inner_from_bytes(&data)
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;
        *self.inner.lock().expect("WorkbookXlsx lock poisoned") = new_inner;
        Ok(())
    }

    /// Read an .xlsx file from disk.  Async.
    ///
    /// @remarks Must be awaited.  The workbook state is only swapped when the
    /// returned Promise resolves.  Accessing worksheets before awaiting the
    /// Promise will see the old (stale) state.
    #[napi]
    pub async fn read_file(&self, path: String) -> Result<()> {
        let p = PathBuf::from(path);
        let new_inner =
            crate::reader::xlsx::workbook_inner_from_path(&p).map_err(|e| napi::Error::from_reason(e.to_string()))?;
        *self.inner.lock().expect("WorkbookXlsx lock poisoned") = new_inner;
        Ok(())
    }

    /// Write the workbook to an .xlsx buffer.  Async.
    ///
    /// Clones the workbook state briefly under the lock, then builds the
    /// .xlsx archive outside the lock (calamine / zip I/O is expensive).
    ///
    /// @remarks Must be awaited.  The returned Promise resolves to a Buffer
    /// of the .xlsx data.
    #[napi]
    pub async fn write(&self) -> Result<Buffer> {
        let inner = self.inner.lock().expect("Workbook lock poisoned").clone();
        let bytes =
            crate::writer::xlsx::workbook_to_bytes(&inner).map_err(|e| napi::Error::from_reason(e.to_string()))?;
        Ok(Buffer::from(bytes))
    }

    /// Write the workbook to an .xlsx file on disk.  Async.
    ///
    /// @remarks Must be awaited.  The file is only fully written when the
    /// returned Promise resolves.
    #[napi]
    pub async fn write_file(&self, path: String) -> Result<()> {
        let inner = self.inner.lock().expect("Workbook lock poisoned").clone();
        let p = PathBuf::from(path);
        crate::writer::xlsx::workbook_to_path(&inner, &p).map_err(|e| napi::Error::from_reason(e.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workbook_xlsx_new_shares_arc() {
        let inner = Arc::new(Mutex::new(WorkbookInner::new()));
        let wb = WorkbookXlsx::new(Arc::clone(&inner));
        // Mutate through the Xlsx handle
        {
            let mut g = wb.inner.lock().unwrap();
            g.add_worksheet("FromXlsx".into());
        }
        // Verify through the original Arc
        let g = inner.lock().unwrap();
        assert_eq!(g.worksheet_count(), 1);
        assert_eq!(g.worksheets()[0].name(), "FromXlsx");
    }

    #[test]
    fn test_workbook_xlsx_read_swaps_inner() {
        let inner = Arc::new(Mutex::new(WorkbookInner::new()));
        let wb = WorkbookXlsx::new(Arc::clone(&inner));

        // Initially the inner has 0 sheets
        assert_eq!(wb.inner.lock().unwrap().worksheet_count(), 0);

        // Build a new inner via the reader and swap
        let bytes = make_test_xlsx_bytes();
        let new_inner = crate::reader::xlsx::workbook_inner_from_bytes(&bytes).unwrap();
        *wb.inner.lock().unwrap() = new_inner;

        // Now the handle's shared state has been replaced
        let g = inner.lock().unwrap();
        assert_eq!(g.worksheet_count(), 1);
        assert_eq!(g.worksheets()[0].name(), "Sheet1");
    }

    #[test]
    fn test_workbook_xlsx_write_to_buffer() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let inner = Arc::new(Mutex::new(WorkbookInner::new()));
        // Add a sheet so we have something to write
        inner.lock().unwrap().add_worksheet("WriteTest".into());
        let wb = WorkbookXlsx::new(Arc::clone(&inner));

        let buffer = rt.block_on(wb.write()).unwrap();
        assert!(!buffer.is_empty(), "buffer should not be empty");

        // Verify the bytes produce a valid workbook
        let re_read = crate::reader::xlsx::workbook_inner_from_bytes(&buffer[..]).unwrap();
        assert_eq!(re_read.worksheet_count(), 1);
        assert!(re_read.worksheets().iter().any(|ws| ws.name() == "WriteTest"));
    }

    #[test]
    fn test_workbook_xlsx_write_file() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let inner = Arc::new(Mutex::new(WorkbookInner::new()));
        inner.lock().unwrap().add_worksheet("FileWrite".into());
        let wb = WorkbookXlsx::new(inner);

        let tmp = std::env::temp_dir().join(format!(
            "excelrs_xlsx_write_{}.xlsx",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        let tmp_str = tmp.to_string_lossy().to_string();

        rt.block_on(wb.write_file(tmp_str)).unwrap();
        assert!(tmp.exists(), "written file should exist");

        // Clean up
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_workbook_xlsx_async_read_swaps_state_via_block_on() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let inner = Arc::new(Mutex::new(WorkbookInner::new()));
        let wb = WorkbookXlsx::new(Arc::clone(&inner));

        // Confirm initial state
        assert_eq!(wb.inner.lock().unwrap().worksheet_count(), 0);

        let bytes = make_test_xlsx_bytes();
        let buf = Buffer::from(bytes.as_slice());

        // block_on drives the async read to completion
        rt.block_on(wb.read(buf)).unwrap();

        // After block_on, state is swapped (same as JS await)
        assert_eq!(inner.lock().unwrap().worksheet_count(), 1);
        assert_eq!(inner.lock().unwrap().worksheets()[0].name(), "Sheet1");
    }

    #[test]
    fn test_workbook_xlsx_async_write_produces_buffer_via_block_on() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let mut inner = WorkbookInner::new();
        inner.add_worksheet("AsyncTest".into());
        let inner = Arc::new(Mutex::new(inner));
        let wb = WorkbookXlsx::new(Arc::clone(&inner));

        let buffer = rt.block_on(wb.write()).unwrap();
        assert!(!buffer.is_empty());

        // Re-read to confirm the data is valid
        let re_read = crate::reader::xlsx::workbook_inner_from_bytes(&buffer[..]).unwrap();
        assert_eq!(re_read.worksheet_count(), 1);
        assert!(re_read.worksheets().iter().any(|ws| ws.name() == "AsyncTest"));
    }

    #[test]
    fn test_roundtrip_header_footer_and_page_setup() {
        use crate::model::header_footer::HeaderFooter;
        use crate::model::page_setup::{PageMargins, PageSetup};

        let mut inner = WorkbookInner::new();
        let mut ws = inner.add_worksheet("Sheet1".into());
        ws.set_header_footer(Some(HeaderFooter {
            odd_header: Some("&CSheet&RPage".into()),
            odd_footer: Some("&P of &N".into()),
            ..Default::default()
        }));
        ws.set_page_setup(Some(PageSetup {
            orientation: Some("landscape".into()),
            paper_size: Some(9),
            margins: Some(PageMargins {
                left: Some(0.5),
                right: Some(0.5),
                top: Some(1.0),
                bottom: Some(1.0),
                header: Some(0.5),
                footer: Some(0.5),
            }),
            ..Default::default()
        }));

        let bytes = crate::writer::xlsx::workbook_to_bytes(&inner).unwrap();
        let re = crate::reader::xlsx::workbook_inner_from_bytes(&bytes).unwrap();
        let rws = &re.worksheets()[0];

        let hf = rws.header_footer().expect("headerFooter should round-trip");
        assert_eq!(hf.odd_header.as_deref(), Some("&CSheet&RPage"));
        assert_eq!(hf.odd_footer.as_deref(), Some("&P of &N"));

        let ps = rws.page_setup().expect("pageSetup should round-trip");
        assert_eq!(ps.orientation.as_deref(), Some("landscape"));
        assert_eq!(ps.paper_size, Some(9));
        let m = ps.margins.expect("margins should round-trip");
        assert_eq!(m.top, Some(1.0));
        assert_eq!(m.left, Some(0.5));
    }

    #[test]
    fn test_roundtrip_workbook_views_and_calc() {
        use crate::model::workbook_view::{CalcProperties, WorkbookView};

        let mut inner = WorkbookInner::new();
        inner.add_worksheet("Sheet1".into());
        inner.set_views(vec![WorkbookView {
            active_tab: Some(1),
            visibility: Some("visible".into()),
            minimized: Some(true),
            ..Default::default()
        }]);
        inner.set_calc_properties(Some(CalcProperties {
            full_calc_on_load: Some(true),
            calc_id: Some(124519),
            ..Default::default()
        }));

        let bytes = crate::writer::xlsx::workbook_to_bytes(&inner).unwrap();
        let re = crate::reader::xlsx::workbook_inner_from_bytes(&bytes).unwrap();
        let views = re.views();
        assert_eq!(views.len(), 1);
        assert_eq!(views[0].active_tab, Some(1));
        assert_eq!(views[0].minimized, Some(true));
        let calc = re.calc_properties().expect("calcProperties should round-trip");
        assert_eq!(calc.full_calc_on_load, Some(true));
        assert_eq!(calc.calc_id, Some(124519));
    }

    #[test]
    fn test_roundtrip_comments() {
        use crate::model::comment::CellComment;

        let mut inner = WorkbookInner::new();
        let ws = inner.add_worksheet("Sheet1".into());
        ws.insert_cell_comment(
            1,
            1,
            CellComment {
                text: "a note".into(),
                author: Some("Alice".into()),
            },
        );

        let bytes = crate::writer::xlsx::workbook_to_bytes(&inner).unwrap();
        let re = crate::reader::xlsx::workbook_inner_from_bytes(&bytes).unwrap();
        let rws = &re.worksheets()[0];

        let c = rws.get_cell_by_rc(1, 1).comment().expect("comment should round-trip");
        assert_eq!(c.text, "a note");
        assert_eq!(c.author.as_deref(), Some("Alice"));
    }

    #[test]
    fn test_roundtrip_images() {
        use crate::model::image::{AddImageOptions, ImageAnchor};

        let mut inner = WorkbookInner::new();
        let ws = inner.add_worksheet("Sheet1".into());
        let anchor = ImageAnchor {
            anchor_type: "oneCell".into(),
            col: 1,
            row: 2,
            x: 0,
            y: 0,
            col2: 0,
            row2: 0,
            x2: 0,
            y2: 0,
        };
        let _ = ws.add_image(AddImageOptions {
            extension: "png".into(),
            buffer: vec![1, 2, 3, 4, 5],
            image_type: None,
            positioning: Some("oneCell".into()),
            anchor,
        });

        let bytes = crate::writer::xlsx::workbook_to_bytes(&inner).unwrap();
        let re = crate::reader::xlsx::workbook_inner_from_bytes(&bytes).unwrap();
        let rws = &re.worksheets()[0];

        let imgs = rws.get_images();
        assert_eq!(imgs.len(), 1);
        assert_eq!(imgs[0].extension, "png");
        assert_eq!(imgs[0].buffer, vec![1, 2, 3, 4, 5]);
        assert_eq!(imgs[0].anchor.col, 1);
        assert_eq!(imgs[0].anchor.row, 2);
    }

    // ---- helpers ----

    fn make_test_xlsx_bytes() -> Vec<u8> {
        use std::io::Write;

        let mut buf = Vec::new();
        {
            let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
            let options: zip::write::FileOptions<'_, ()> =
                zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

            zip.start_file("[Content_Types].xml", options).unwrap();
            write!(
                zip,
                r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>
  <Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>
  <Override PartName="/xl/sharedStrings.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sharedStrings+xml"/>
</Types>"#
            )
            .unwrap();

            zip.start_file("_rels/.rels", options).unwrap();
            write!(
                zip,
                r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/>
</Relationships>"#
            )
            .unwrap();

            zip.start_file("xl/workbook.xml", options).unwrap();
            write!(
                zip,
                r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
          xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <sheets>
    <sheet name="Sheet1" sheetId="1" r:id="rId1"/>
  </sheets>
</workbook>"#
            )
            .unwrap();

            zip.start_file("xl/_rels/workbook.xml.rels", options).unwrap();
            write!(
                zip,
                r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/>
</Relationships>"#
            )
            .unwrap();

            zip.start_file("xl/worksheets/sheet1.xml", options).unwrap();
            write!(
                zip,
                r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <sheetData>
    <row r="1">
      <c r="A1" t="inlineStr"><is><t>hello</t></is></c>
    </row>
  </sheetData>
</worksheet>"#
            )
            .unwrap();

            zip.start_file("xl/sharedStrings.xml", options).unwrap();
            write!(
                zip,
                r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<sst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" count="0" uniqueCount="0"/>
"#
            )
            .unwrap();

            zip.finish().unwrap();
        }
        buf
    }

    #[test]
    fn test_make_test_xlsx_bytes_is_valid() {
        let bytes = make_test_xlsx_bytes();
        assert!(!bytes.is_empty());
        let inner = crate::reader::xlsx::workbook_inner_from_bytes(&bytes).unwrap();
        assert_eq!(inner.worksheet_count(), 1);
        assert_eq!(inner.worksheets()[0].name(), "Sheet1");
    }
}
