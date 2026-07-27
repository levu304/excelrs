## Context

Internal storage is `WorksheetImage { anchor: ImageAnchor, ext_size: Option<(u32,u32)> }`.
`add_image` populates `ext_size` from the public `ImageAnchorInput.ext`
(width/height px → EMU). The writer emits `<xdr:ext cx=".." cy=".."/>` only for
`oneCellAnchor` when `ext_size` is `Some`.

The reader (`parse_drawing_xml`) only parses `xdr:from` / `xdr:to` markers and
`a:blip`, then hardcodes `ext_size: None` in the caller. So a one-cell image
written by excelrs (or Excel/ExcelJS) is read back without its size, producing
an invalid `ImageAnchorInput` from `getImages()` and corrupt OOXML on re-save.

## Goals / Non-Goals

**Goals:**

- Preserve one-cell image size across read → write (round-trip).
- `getImages()` returns a valid `ext`-shaped anchor for one-cell images read
  from a file.
- Always emit schema-valid `oneCellAnchor` (with `<xdr:ext>`) regardless of
  internal `ext_size` state.

**Non-Goals:**

- Changing the public `ImageAnchorInput` / `ImageInfo` shape (set by PR #39).
- Parsing `xdr:ext` inside `twoCellAnchor` for semantic use (Excel places `ext`
  there only with `editAs="oneCell"`; we capture-but-ignore to avoid stale bleed).
- Changing two-cell anchor behavior (no `ext` in a valid two-cell anchor).

## Decisions

### D1 — Extend `parse_drawing_xml` return to carry `ext_size`

Change the return from `Vec<(String, ImageAnchor)>` to
`Vec<(String, ImageAnchor, Option<(u32,u32)>)>`. The function is private, so the
richer tuple has zero external blast radius and is the cleanest carrier (avoids
polluting `ImageAnchor`, which is a `#[napi(object)]` public struct).

### D2 — Parse `<xdr:ext>` attributes

In the existing `Start | Empty` match arm, add:

```rust
b"xdr:ext" => {
    let mut cx = 0u32;
    let mut cy = 0u32;
    for attr in e.attributes().flatten() {
        match attr.key.as_ref() {
            b"cx" => cx = parse_u32_bytes(&attr.value),
            b"cy" => cy = parse_u32_bytes(&attr.value),
            _ => {}
        }
    }
    current_ext = Some((cx, cy));
}
```

`xdr:ext` is self-closing in Excel/ExcelJS output (`<xdr:ext cx=".." cy=".."/>`),
so both `Event::Start` and `Event::Empty` are covered by the existing arm.
Reset `current_ext = None` whenever `xdr:oneCellAnchor` / `xdr:twoCellAnchor`
start, mirroring how `cur` is reset. Push `current_ext` alongside `(rid, anchor)`
at `xdr:pic` end.

### D3 — Reader caller assigns parsed ext

In `parse_sheet_images`, replace the hardcoded `ext_size: None` with the third
tuple element returned by `parse_drawing_xml`. Caller loop becomes:

```rust
for (rid, anchor, ext) in parse_drawing_xml(&xml) {
    if let Some(target) = media_map.get(&rid) {
        // ... read buffer ...
        imgs.push(WorksheetImage { .., anchor, ext_size: ext, .. });
    }
}
```

### D4 — Writer always emits `<xdr:ext>` for oneCellAnchor

Replace the `if let Some((cx, cy)) = img.ext_size` guard with an unconditional
emit keyed on anchor type, with a `cx="0" cy="0"` fallback:

```rust
if a.anchor_type == AnchorType::OneCell {
    let (cx, cy) = img.ext_size.unwrap_or((0, 0));
    body.push_str(&format!("<xdr:ext cx=\"{cx}\" cy=\"{cy}\"/>"));
}
```

This guarantees valid OOXML even for malformed input files where the source had
no `<xdr:ext>`. Two-cell anchors keep their existing `<xdr:to>` branch and emit
no `ext`.

### D5 — `to_exceljs_shape` is already correct

`ImageAnchor::to_exceljs_shape(ext_size)` already returns `ext: Some(width,
height)` when `ext_size` is `Some`, and `br` only for `TwoCell`. After D1–D3,
well-formed files yield `Some`, so `getImages()` returns a valid shape with no
change to this function. The only residual path returning `br: None, ext: None`
is a source file genuinely missing `<xdr:ext>`; re-saving it triggers D4's
fallback, healing the file on the next round-trip.

## Risks / Trade-offs

- **Container-form `<xdr:ext>`**: some non-Excel tools emit
  `<xdr:ext><a:ext cx=".." cy=".."/></xdr:ext>`. D2 only reads attributes on the
  `xdr:ext` element, so this variant would be missed. Excel and ExcelJS both use
  the self-closing form, so this is out of scope; can be added later if needed.
- **Two-cell with `editAs="oneCell"` + ext**: D2 may capture `ext` for a
  `TwoCell` anchor, but D3 stores it as `ext_size` which `to_exceljs_shape`
  ignores for `AnchorType::TwoCell`. Harmless; the `br` path is authoritative.
- **Zero-size fallback**: D4 emitting `cx="0" cy="0"` for a source with no ext
  makes the image effectively invisible in Excel, but keeps the file openable —
  preferable to corrupt output. Acceptable trade-off; documented in tests.
