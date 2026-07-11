//! Workbook-level parser — reads `xl/workbook.xml` for defined names.
//!
//! Calamine does not expose `<definedNames>`, so we parse the workbook XML
//! directly from the zip archive to recover `<definedName>` entries.

use std::io::{Cursor, Read};

use quick_xml::events::Event;
use quick_xml::Reader as XmlReader;

use crate::error::ExcelrsError;
use crate::model::defined_name::DefinedName;

/// Parse `<definedName>` entries from `xl/workbook.xml`.
///
/// Returns an empty `Vec` when no `<definedNames>` element exists.
/// `sheet_names` is the ordered list of sheet names (from calamine or
/// from `<sheets>`) used to resolve `localSheetId` → sheet name.
pub fn parse_defined_names(data: &[u8], sheet_names: &[String]) -> Result<Vec<DefinedName>, ExcelrsError> {
    let cursor = Cursor::new(data);
    let mut archive = zip::ZipArchive::new(cursor).map_err(|e| ExcelrsError::Zip(e.to_string()))?;

    let mut buf = Vec::new();
    let mut entry = match archive.by_name("xl/workbook.xml") {
        Ok(e) => e,
        Err(_) => return Ok(Vec::new()),
    };
    entry.read_to_end(&mut buf)?;
    let xml = String::from_utf8_lossy(&buf);

    let mut reader = XmlReader::from_str(&xml);
    reader.config_mut().trim_text(true);

    let mut names = Vec::new();
    let mut in_defined_names = false;
    let mut in_defined_name = false;
    let mut current_name = String::new();
    let mut current_value = String::new();
    let mut current_sheet_id: Option<u32> = None;
    let mut depth = 0u32;

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) => {
                let qn = e.name();
                let tag = qn.as_ref();
                if tag == b"definedNames" {
                    in_defined_names = true;
                    depth = 0;
                } else if tag == b"definedName" && in_defined_names {
                    in_defined_name = true;
                    current_name.clear();
                    current_value.clear();
                    current_sheet_id = None;
                    for attr in e.attributes().flatten() {
                        let key = attr.key.as_ref();
                        if key == b"name" {
                            current_name = attr.unescape_value().unwrap_or_default().to_string();
                        }
                        if key == b"localSheetId" {
                            if let Ok(id) = attr.unescape_value().unwrap_or_default().parse::<u32>() {
                                current_sheet_id = Some(id);
                            }
                        }
                    }
                    depth = 0;
                } else if in_defined_name {
                    depth += 1;
                }
            }
            Ok(Event::Text(ref e)) if in_defined_name => {
                if let Ok(text) = e.unescape() {
                    current_value.push_str(&text);
                }
            }
            Ok(Event::End(ref e)) => {
                let qn = e.name();
                let tag = qn.as_ref();
                if tag == b"definedNames" {
                    in_defined_names = false;
                } else if tag == b"definedName" && in_defined_name {
                    in_defined_name = false;
                    let sheet = current_sheet_id.and_then(|id| sheet_names.get(id as usize).cloned());
                    if !current_name.is_empty() {
                        names.push(DefinedName {
                            name: current_name.clone(),
                            value: current_value.trim().to_string(),
                            sheet,
                        });
                    }
                } else if in_defined_name && depth > 0 {
                    depth -= 1;
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(ExcelrsError::Parse(format!("Failed to parse xl/workbook.xml: {e}"))),
            _ => {}
        }
    }

    Ok(names)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_workbook_xml(defined_names_xml: &str) -> Vec<u8> {
        use std::io::Write;
        let mut buf = Vec::new();
        {
            let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
            let opts: zip::write::FileOptions<'_, ()> =
                zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

            zip.start_file("[Content_Types].xml", opts).unwrap();
            write!(
                zip,
                r#"<?xml version="1.0"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>
</Types>"#
            )
            .unwrap();

            zip.start_file("xl/workbook.xml", opts).unwrap();
            write!(
                zip,
                r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
          xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <sheets>
    <sheet name="Sheet1" sheetId="1" r:id="rId1"/>
    <sheet name="Data" sheetId="2" r:id="rId2"/>
  </sheets>
  {defined_names_xml}
</workbook>"#
            )
            .unwrap();

            zip.finish().unwrap();
        }
        buf
    }

    #[test]
    fn test_parse_defined_names_global() {
        let data = make_workbook_xml(r#"<definedNames><definedName name="TaxRate">0.08</definedName></definedNames>"#);
        let names = parse_defined_names(&data, &["Sheet1".into(), "Data".into()]).unwrap();
        assert_eq!(names.len(), 1);
        assert_eq!(names[0].name, "TaxRate");
        assert_eq!(names[0].value, "0.08");
        assert!(names[0].sheet.is_none());
    }

    #[test]
    fn test_parse_defined_names_sheet_scoped() {
        let data = make_workbook_xml(
            r#"<definedNames><definedName name="LocalRef" localSheetId="0">$A$1:$B$10</definedName></definedNames>"#,
        );
        let names = parse_defined_names(&data, &["Sheet1".into(), "Data".into()]).unwrap();
        assert_eq!(names.len(), 1);
        assert_eq!(names[0].name, "LocalRef");
        assert_eq!(names[0].value, "$A$1:$B$10");
        assert_eq!(names[0].sheet, Some("Sheet1".into()));
    }

    #[test]
    fn test_parse_defined_names_local_id_out_of_range() {
        let data = make_workbook_xml(
            r#"<definedNames><definedName name="Bad" localSheetId="99">x</definedName></definedNames>"#,
        );
        let names = parse_defined_names(&data, &["Sheet1".into()]).unwrap();
        assert_eq!(names.len(), 1);
        assert_eq!(names[0].name, "Bad");
        assert!(names[0].sheet.is_none());
    }

    #[test]
    fn test_parse_defined_names_empty() {
        let data = make_workbook_xml("");
        let names = parse_defined_names(&data, &["Sheet1".into()]).unwrap();
        assert!(names.is_empty());
    }

    #[test]
    fn test_parse_defined_names_missing_xl_workbook_xml() {
        // Zip without xl/workbook.xml
        let data = {
            use std::io::Write;
            let mut buf = Vec::new();
            {
                let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
                let opts: zip::write::FileOptions<'_, ()> =
                    zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
                zip.start_file("[Content_Types].xml", opts).unwrap();
                write!(zip, r#"<?xml version="1.0"?><Types xmlns="..."/> "#).unwrap();
                zip.finish().unwrap();
            }
            buf
        };
        // Invalid zip-entry parse → should just return empty vec
        let names = parse_defined_names(&data, &["Sheet1".into()]).unwrap();
        assert!(names.is_empty());
    }
}
