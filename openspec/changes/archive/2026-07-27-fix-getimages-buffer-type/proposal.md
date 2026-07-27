## Why

PR #38 (`fix-addimage-buffer-type`) introduced `NapiBuffer` so `addImage` accepts
a Node.js `Buffer` (and still `number[]`) at runtime. It declared
`ImageInfo.buffer` as `Buffer` in TypeScript via `#[napi(ts_type = "Buffer")]`,
but `NapiBuffer`'s `ToNapiValue` still returns `Array<number>` at runtime — the
"safe lie" noted in `src/model/image.rs`.

This violates the `images` spec, which requires `getImages()[0].buffer` to be a
real `Buffer` at runtime ("buffer is a `Buffer` matching the input bytes"). The
lie is also actively harmful: consumers calling `img.buffer.toString("base64")`
or `fs.writeFileSync(path, img.buffer)` crash at runtime, and it breaks
ExcelJS parity (ExcelJS returns a real `Buffer`).

Self-review plus the napi.rs typed-array docs confirm the lie is unnecessary:
`Buffer` can be created from `Vec<u8>` **zero-copy** via an external buffer whose
finalizer frees the `Vec<u8>` on GC, falling back to a copy on runtimes that
reject external buffers (e.g. Electron). So we can return a real `Buffer` and
satisfy the spec with no extra copying.

## What Changes

- `src/model/image.rs`: `NapiBuffer`'s `ToNapiValue` returns a real Node.js
  `Buffer` (`val.0.into()` → `<Buffer as ToNapiValue>::to_napi_value`). The
  input path (`FromNapiValue`) is unchanged — it still accepts `Buffer` and
  `number[]`. Remove the "safe lie" comment.
- Add a JS test asserting `Buffer.isBuffer(ws.getImages()[0].buffer)` and that
  the bytes match the input.

## Impact

| Area | Impact |
| ------ | -------- |
| `src/model/image.rs` | `NapiBuffer::to_napi_value` returns `Buffer` (not `Array<number>`); remove the "safe lie" comment. |
| `index.d.ts` / `native.d.ts` | No type change (`buffer` already `Buffer`). Only regenerated if `napi build` re-runs. |
| Tests | Add a JS test for `getImages()` `buffer` runtime type. |
| Spec | No change needed — the `images` spec already requires a runtime `Buffer`; this change implements it. |
