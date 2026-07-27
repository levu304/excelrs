## Context

`AddImageOptions` and `ImageInfo` are `#[napi(object)]` structs in `src/model/image.rs`. napi-rs maps `Vec<u8>` fields to `Array<number>` in the generated TypeScript declarations. At runtime, napi-rs accepts any `TypedArray` (including `Buffer`/`Uint8Array`) and extracts the bytes — so runtime works. But TypeScript sees `Buffer` ≠ `number[]` and rejects the call.

Two structs affected:

- `AddImageOptions` (input to `ws.addImage()`) — user-facing
- `ImageInfo` (return from `ws.getImages()`) — user-facing

Internal `WorksheetImage` (not napi-annotated) keeps `buffer: Vec<u8>` — no change needed there.

## Goals / Non-Goals

**Goals:**

- `AddImageOptions.buffer` accepts `Buffer` in TS (not just `Array<number>`)
- `ImageInfo.buffer` returns `Buffer` in TS (not just `Array<number>`)
- Runtime stays compatible with both `Buffer` and `number[]` callers
- Generated `index.d.ts` / `native.d.ts` reflect the correct types after `napi build`

**Non-Goals:**

- Not changing any other `Vec<u8>` napi fields — only image-related
- Not changing the runtime behavior or internal storage types
- Not adding `stream` or `path` alternatives to `AddImageOptions.buffer` (ExcelJS supports those, tracked separately)

## Decisions

### D1 — Use `napi::bindgen_prelude::Buffer` instead of `Vec<u8>`

napi-rs maps `Buffer` to Node.js `Buffer` in the generated `.d.ts`. At runtime `Buffer` is a `Uint8Array` — the correct type for binary data in Node.js.

**Alternatives considered:**

- **`Vec<u8>` (current)**: Generates `Array<number>`. Wrong type. Rejected.
- **Custom `#[napi]` fn with `Buffer` arg**: Would require changing `addImage()` signature from accepting an object to having separate args. Breaking change. Rejected.
- **Type-level override in `index.d.ts`**: Would require post-build patching that breaks on every `napi build`. Fragile. Rejected.

**Chosen**: `#[napi(ts_type = "Buffer")]` on `Vec<u8>` fields — generates `Buffer` in `.d.ts`, keeps `Vec<u8>` at Rust level (so `Clone` works).

### D2 — `add_image` accepts raw `Object` for runtime Buffer support

`#[napi(ts_type = "Buffer")]` on `Vec<u8>` only changes the **TypeScript declaration** — napi-rs codegen still generates runtime conversion from `Vec<u8>` which rejects `Buffer` (only accepts `Array<number>`).

For **input** (`add_image`): Accept `napi::JsObject` as the parameter (not `AddImageOptions` struct) so the napi-rs auto-conversion layer doesn't gatekeep the buffer field. Manually extract all fields using `Object::get()`, and extract buffer via a helper that tries `Buffer::from_napi_value` first (handles Buffer/TypedArray), then falls back to `Vec<u8>::from_napi_value` (handles `Array<number>`). Uses `#[napi(ts_type = "AddImageOptions")]` on the `Object` parameter to preserve the TS type declaration.

For **output** (`get_images`): Keep `ImageInfo` as `#[napi(object)]` with `Vec<u8>` + `ts_type = "Buffer"`. TS says `Buffer`, but runtime returns `Array<number>`. This is a safe lie — `Buffer.from(array)` works everywhere and there's no regression from the pre-existing behavior.

### D3 — No sweep for other `Vec<u8>` napi fields

Only `AddImageOptions` and `ImageInfo` have `Vec<u8>` in napi object structs. The streaming types (`JsStreamValue`, etc.) don't carry raw bytes. `rowBreaks`/`colBreaks` use `Vec<u32>` — correct as `Array<number>`. No other fields affected.

## Risks / Trade-offs

| Risk | Likelihood | Mitigation |
| --- | --- | --- |
| Existing callers passing `number[]` break | Very low | The custom `extract_buffer` helper tries `Buffer` first, then falls back to `Vec<u8>` — both paths produce `Vec<u8>`. |
| `napi build` reversion on next upgrade | Low | Both `ts_type` annotations and `Object` parameter survive `napi build`. Only the generated `.d.ts` files change. |
| `get_images()` returns `Array<number>` not `Buffer` at runtime | Low | TS type says `Buffer` but runtime returns `Array`. Users can wrap with `Buffer.from()`. Not a regression from current behavior. |
