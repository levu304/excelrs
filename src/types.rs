//! Shared utilities for address parsing, coordinate math, and column letter conversion.
//!
//! # Address format
//! Excel uses A1-style references: column letter(s) + row number.
//! - Columns: A–Z (1–26), AA–AZ (27–52), ... up to XFD (16384)
//! - Rows: 1–1048576
//!
//! All coordinates are 1-indexed, matching exceljs and Excel's internal model.

/// Maximum number of columns supported by Excel (XFD = 16384).
pub const MAX_COL: u32 = 16_384;

/// Maximum number of rows supported by Excel (1,048,576).
pub const MAX_ROW: u32 = 1_048_576;

/// Minimum valid column number (1-indexed, A).
pub const MIN_COL: u32 = 1;

/// Minimum valid row number (1-indexed).
pub const MIN_ROW: u32 = 1;

/// Convert a column number (1-indexed) to its letter representation.
///
/// ```
/// # use excelrs_core::types::col_num_to_letter;
/// assert_eq!(col_num_to_letter(1).unwrap(), "A");
/// assert_eq!(col_num_to_letter(26).unwrap(), "Z");
/// assert_eq!(col_num_to_letter(27).unwrap(), "AA");
/// assert_eq!(col_num_to_letter(16384).unwrap(), "XFD");
/// ```
pub fn col_num_to_letter(col: u32) -> Result<String, super::error::ExcelrsError> {
    if !(MIN_COL..=MAX_COL).contains(&col) {
        return Err(super::error::ExcelrsError::InvalidAddress(format!(
            "Column out of range: {col} (valid: {MIN_COL}–{MAX_COL})"
        )));
    }
    let mut n = col;
    let mut s = String::new();
    while n > 0 {
        n -= 1;
        s.insert(0, char::from_u32(b'A' as u32 + (n % 26)).unwrap());
        n /= 26;
    }
    Ok(s)
}

/// Convert a column letter string to its 1-indexed number.
///
/// ```
/// # use excelrs_core::types::col_letter_to_num;
/// assert_eq!(col_letter_to_num("A").unwrap(), 1);
/// assert_eq!(col_letter_to_num("Z").unwrap(), 26);
/// assert_eq!(col_letter_to_num("AA").unwrap(), 27);
/// assert_eq!(col_letter_to_num("XFD").unwrap(), 16384);
/// ```
pub fn col_letter_to_num(letters: &str) -> Result<u32, super::error::ExcelrsError> {
    if letters.is_empty() {
        return Err(super::error::ExcelrsError::InvalidAddress(
            "Empty column reference".into(),
        ));
    }
    let mut col: u32 = 0;
    for ch in letters.chars() {
        if !ch.is_ascii_alphabetic() {
            return Err(super::error::ExcelrsError::InvalidAddress(format!(
                "Invalid column character: '{ch}' in '{letters}'"
            )));
        }
        let digit = ch.to_ascii_uppercase() as u32 - b'A' as u32 + 1;
        col = col.checked_mul(26).and_then(|c| c.checked_add(digit)).ok_or_else(|| {
            super::error::ExcelrsError::InvalidAddress(format!("Column overflow: '{letters}' exceeds maximum (XFD)"))
        })?;
    }
    if col > MAX_COL {
        return Err(super::error::ExcelrsError::InvalidAddress(format!(
            "Column out of range: '{letters}' (max: XFD / {MAX_COL})"
        )));
    }
    Ok(col)
}

/// Parse an A1-style cell reference into (column, row) — both 1-indexed.
///
/// ```
/// # use excelrs_core::types::parse_address;
/// let (col, row) = parse_address("A1").unwrap();
/// assert_eq!(col, 1);
/// assert_eq!(row, 1);
///
/// let (col, row) = parse_address("AA42").unwrap();
/// assert_eq!(col, 27);
/// assert_eq!(row, 42);
/// ```
pub fn parse_address(address: &str) -> Result<(u32, u32), super::error::ExcelrsError> {
    if address.is_empty() {
        return Err(super::error::ExcelrsError::InvalidAddress("Empty cell address".into()));
    }

    let col_end = address.find(|c: char| c.is_ascii_digit()).ok_or_else(|| {
        super::error::ExcelrsError::InvalidAddress(format!("Missing row number in address: '{address}'"))
    })?;

    if col_end == 0 {
        return Err(super::error::ExcelrsError::InvalidAddress(format!(
            "Missing column letters in address: '{address}'"
        )));
    }

    let col_part = &address[..col_end];
    let row_part = &address[col_end..];

    let col = col_letter_to_num(col_part)?;
    let row: u32 = row_part.parse().map_err(|_| {
        super::error::ExcelrsError::InvalidAddress(format!("Invalid row number: '{row_part}' in '{address}'"))
    })?;

    if !(MIN_ROW..=MAX_ROW).contains(&row) {
        return Err(super::error::ExcelrsError::InvalidAddress(format!(
            "Row out of range: {row} (valid: {MIN_ROW}–{MAX_ROW})"
        )));
    }

    Ok((col, row))
}

/// Convert (column, row) to an A1-style address string.
///
/// ```
/// # use excelrs_core::types::address_to_string;
/// assert_eq!(address_to_string(1, 1).unwrap(), "A1");
/// assert_eq!(address_to_string(27, 42).unwrap(), "AA42");
/// ```
pub fn address_to_string(col: u32, row: u32) -> Result<String, super::error::ExcelrsError> {
    let col_letter = col_num_to_letter(col)?;
    if !(MIN_ROW..=MAX_ROW).contains(&row) {
        return Err(super::error::ExcelrsError::InvalidAddress(format!(
            "Row out of range: {row} (valid: {MIN_ROW}–{MAX_ROW})"
        )));
    }
    Ok(format!("{col_letter}{row}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_col_letter_to_num_basic() {
        assert_eq!(col_letter_to_num("A").unwrap(), 1);
        assert_eq!(col_letter_to_num("Z").unwrap(), 26);
        assert_eq!(col_letter_to_num("AA").unwrap(), 27);
        assert_eq!(col_letter_to_num("AZ").unwrap(), 52);
        assert_eq!(col_letter_to_num("BA").unwrap(), 53);
    }

    #[test]
    fn test_col_letter_to_num_xfd() {
        assert_eq!(col_letter_to_num("XFD").unwrap(), 16384);
    }

    #[test]
    fn test_col_letter_to_num_lowercase() {
        assert_eq!(col_letter_to_num("aa").unwrap(), 27);
    }

    #[test]
    fn test_col_letter_to_num_errors() {
        assert!(col_letter_to_num("").is_err());
        assert!(col_letter_to_num("123").is_err());
        assert!(col_letter_to_num("XFE").is_err());
    }

    #[test]
    fn test_col_num_to_letter_basic() {
        assert_eq!(col_num_to_letter(1).unwrap(), "A");
        assert_eq!(col_num_to_letter(26).unwrap(), "Z");
        assert_eq!(col_num_to_letter(27).unwrap(), "AA");
        assert_eq!(col_num_to_letter(52).unwrap(), "AZ");
    }

    #[test]
    fn test_col_num_to_letter_xfd() {
        assert_eq!(col_num_to_letter(16384).unwrap(), "XFD");
    }

    #[test]
    fn test_col_num_to_letter_errors() {
        assert!(col_num_to_letter(0).is_err());
        assert!(col_num_to_letter(16385).is_err());
    }

    #[test]
    fn test_parse_address_basic() {
        assert_eq!(parse_address("A1").unwrap(), (1, 1));
        assert_eq!(parse_address("Z100").unwrap(), (26, 100));
        assert_eq!(parse_address("AA42").unwrap(), (27, 42));
        assert_eq!(parse_address("XFD1048576").unwrap(), (16384, 1048576));
    }

    #[test]
    fn test_address_roundtrip() {
        for (col, row) in &[(1, 1), (26, 42), (27, 100), (16384, 1048576), (512, 65536)] {
            let addr = address_to_string(*col, *row).unwrap();
            let (c, r) = parse_address(&addr).unwrap();
            assert_eq!((c, r), (*col, *row), "roundtrip failed for {addr}");
        }
    }

    #[test]
    fn test_parse_address_errors() {
        assert!(parse_address("").is_err());
        assert!(parse_address("1").is_err());
        assert!(parse_address("A").is_err());
        assert!(parse_address("A0").is_err());
        assert!(parse_address("A1048577").is_err());
        assert!(parse_address("XFE1").is_err());
    }

    #[test]
    fn test_address_to_string_errors() {
        assert!(address_to_string(0, 1).is_err());
        assert!(address_to_string(1, 0).is_err());
        assert!(address_to_string(16385, 1).is_err());
        assert!(address_to_string(1, 1048577).is_err());
    }
}
