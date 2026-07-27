## Why

`AddImageOptions.buffer` and `ImageInfo.buffer` accept Node.js `Buffer` at runtime (napi-rs handles typed arrays), but the generated `index.d.ts` declares them as `Array<number>`. TypeScript rejects any call passing a real `Buffer` — TS type error. This breaks every user who reads a file with `readFile` and passes the result as the image buffer.

## What Changes

- **`AddImageOptions.buffer`**: Change napi type from `Vec<u8>` to `Buffer` → generated TS becomes `Buffer` instead of `Array<number>`
- **`ImageInfo.buffer`**: Same fix so `getImages()` returns a typed `Buffer`, not `Array<number>`
- **`add_image()` conversion**: Convert `Buffer` → `Vec<u8>` when storing into internal `WorksheetImage.buffer: Vec<u8>`
- **`ImageInfo` construction**: Convert internal `Vec<u8>` → `Buffer` when returning to JS
- **No breaking runtime change**: `Buffer` is a superset of the current API — existing callers passing `number[]` will still work (napi-rs handles the coercion)

## Capabilities

### New Capabilities

*(none)*

### Modified Capabilities

- `images`: `AddImageOptions.buffer` and `ImageInfo.buffer` change from `Array<number>` to `Buffer` in the TS type declarations. No behavioral change.

## Impact

| Area | Impact |
| --- | --- |
| `src/model/image.rs` | `AddImageOptions.buffer` and `ImageInfo.buffer`: `Vec<u8>` → `Buffer`. Import `napi::bindgen_prelude::Buffer`. |
| `src/model/worksheet.rs` | `add_image()`: `opts.buffer` is now `Buffer` → call `.to_vec()` before storing into `WorksheetImage.buffer: Vec<u8>`. |
| `index.d.ts` | Regenerated — `buffer: Array<number>` → `buffer: Buffer` in both `AddImageOptions` and `ImageInfo`. |
| `native.d.ts` | Same regeneration — `buffer: Array<number>` → `buffer: Buffer`. |
| Tests | Pure-Rust tests construct `Vec<u8>` directly (not through FFI) — no changes needed. Add JS-side type test. |
