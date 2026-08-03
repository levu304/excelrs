#!/usr/bin/env python3
"""Generate fixtures for the cached-formula round-trip change.

Fixture 1 (ExcelJS-authored): generated via Node+ExcelJS so the committed
bytes genuinely exercise ExcelJS's `model.result` dispatch. See
gen-exceljs-fixture.js.

Fixture 2 (hand-crafted): a ~200-byte minimal .xlsx built by hand here,
isolated from both writers. Carries `<f>A2+B2</f><v>3</v>`.
"""
import struct
import zipfile
from pathlib import Path

CHANGE_DIR = Path(__file__).resolve().parents[1]
OPENSPEC_FIXTURES = CHANGE_DIR / "openspec" / "changes" / "formula-cached-value-round-trip" / "fixtures"
TEST_FIXTURES = CHANGE_DIR / "__test__" / "fixtures"

OPENSPEC_FIXTURES.mkdir(parents=True, exist_ok=True)
TEST_FIXTURES.mkdir(parents=True, exist_ok=True)

# ---- Minimal OOXML pieces for a single-cell workbook (A1 has <f>...<v>) ----
# SheetXML: A1 = formula "A2+B2" with cached result 3. We also add B1=1,
# B2=2 so the formula "A2+B2" is semantically meaningful.
SHEET_XML = (
    '<?xml version="1.0"?>'
    '<?mso-application progid="Excel.Sheet"?>'
    '<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">'
    '<dimension ref="A1:B2"/>'
    '<sheetData>'
    '<c r="B1" t="n"><v>1</v></c>'
    '<c r="B2" t="n"><v>2</v></c>'
    '<c r="A1"><f>A2+B2</f><v>3</v></c>'
    '</sheetData>'
    '</worksheet>'
)

WORKBOOK_XML = (
    '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
    '<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"'
    ' xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"'
    ' r:id="rId1">'
    '<sheets><sheet name="Sheet1" sheetId="1" r:id="rId1"/></sheets>'
    '</workbook>'
)

APP_XML = (
    '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
    '<properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/extended-properties">'
    '<Application>excelrs-handcrafted</Application>'
    '<DocSecurity>0</DocSecurity>'
    '<ScaleCrop>false</ScaleCrop>'
    '<Version>1</Version>'
    '</properties>'
)

CONTENT_TYPES_XML = (
    '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
    '<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">'
    '<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>'
    '<Default Extension="xml" ContentType="application/xml"/>'
    '<Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>'
    '<Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>'
    '</Types>'
)

# Core relationships for the workbook part -> worksheet.
WORKBOOK_RELS = """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/>
</Relationships>
"""

# Root-level _rels/.rels -> xl/workbook.xml
ROOT_RELS = """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/>
</Relationships>
"""

# Shared strings (empty table — not used here but calamine may expect the part).
SST_XML = (
    '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
    '<sst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" count="0" uniqueCount="0"></sst>'
)

STYLES_XML = (
    '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
    '<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">'
    '<fonts count="1"><font><sz val="11"/><color indexed="8"/><name val="Calibri"/></font></fonts>'
    '<fills count="2"><fill><foreground/></fill><fill><foreground/></fill></fills>'
    '<borders count="1"><border/></borders>'
    '<cellStyleXfs count="1"><xf/></cellStyleXfs>'
    '<cellXfs count="1"><xf/></cellXfs>'
    '</styleSheet>'
)


def build_handcrafted_xlsx(out_path: Path):
    """Write a minimal valid .xlsx with zipfile (stored, no compression)."""
    with zipfile.ZipFile(out_path, "w", zipfile.ZIP_DEFLATED) as z:
        z.writestr("[Content_Types].xml", CONTENT_TYPES_XML)
        z.writestr("_rels/.rels", ROOT_RELS)
        z.writestr("xl/workbook.xml", WORKBOOK_XML)
        z.writestr("xl/_rels/workbook.xml.rels", WORKBOOK_RELS)
        z.writestr("xl/worksheets/sheet1.xml", SHEET_XML)
        z.writestr("xl/sharedStrings.xml", SST_XML)
        z.writestr("xl/styles.xml", STYLES_XML)
        z.writestr("docProps/app.xml", APP_XML)
    print(f"wrote {out_path} ({out_path.stat().st_size} bytes)")


def copy_into_test_fixtures(src: Path):
    dst = TEST_FIXTURES / src.name
    dst.write_bytes(src.read_bytes())
    print(f"copied {src.name} -> {dst}")


if __name__ == "__main__":
    # Fixture 2: hand-crafted (built here from raw XML).
    hand = OPENSPEC_FIXTURES / "hand-cached-formula.xlsx"
    build_handcrafted_xlsx(hand)
    copy_into_test_fixtures(hand)