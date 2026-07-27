## 1. Rust source changes

## 1. Rust source: TypeScript declaration fix

- [x] 1.1 Add `#[napi(ts_type = "Buffer")]` to `AddImageOptions.buffer` and `ImageInfo.buffer` in `src/model/image.rs`
- [x] 1.2 `src/model/image.rs` compiles clean

## 2. Rust source: Runtime Buffer support

- [x] 2.1 Add `NapiBuffer(Vec<u8>)` wrapper type with `FromNapiValue` (accepts Buffer/TypedArray + Array<number>) and `ToNapiValue`
- [x] 2.2 Change `AddImageOptions.buffer` and `ImageInfo.buffer` from `Vec<u8>` to `NapiBuffer`
- [x] 2.3 Update `add_image()` to unwrap `opts.buffer.0`, `get_images()` to wrap `NapiBuffer(img.buffer)`
- [x] 2.4 Update tests in `handle.rs` to use `NapiBuffer(vec![...])` and compare `.0`

## 3. Rebuild and verify

- [x] 3.1 Run `napi build` to regenerate `index.js`, `index.d.ts`, `native.js`, `native.d.ts`
- [x] 3.2 Verify `index.d.ts` shows `buffer: Buffer` (not `Array<number>`) in both `AddImageOptions` and `ImageInfo`
- [x] 3.3 Run `cargo test` — 384 Rust tests pass
- [x] 3.4 Run `pnpm test` — 135 JS tests pass

## 4. Validation

- [x] 4.1 Verify `ws.addImage({ buffer: Buffer.from([...]), ... })` works at runtime
- [x] 4.2 Verify backward compat: `ws.addImage({ buffer: [1,2,3], ... })` still works at runtime
