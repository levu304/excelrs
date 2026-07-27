## Approach

Change **only** `NapiBuffer`'s `ToNapiValue` (the output path). The input path
(`FromNapiValue`, accepting `Buffer` + `number[]`) stays as-is from PR #38.

```rust
impl ToNapiValue for NapiBuffer {
    unsafe fn to_napi_value(env: sys::napi_env, val: Self) -> napi::Result<sys::napi_value> {
        let buf: Buffer = val.0.into();
        unsafe { <Buffer as ToNapiValue>::to_napi_value(env, buf) }
    }
}
```

`val.0` moves the `Vec<u8>` into a `Buffer`; `Buffer::to_napi_value` transfers
the allocation to a JS `Buffer` (zero-copy external buffer) with a finalizer
that drops the `Vec<u8>` when JS garbage-collects it.

## Rationale (napi.rs typed-array docs)

- "A `Buffer` can be created from `Vec<u8>`."
- "Where the runtime permits external buffers, NAPI-RS transfers the allocation
  to the JavaScript `Buffer` without a copy, and its finalizer releases the
  `Vec<u8>` after JavaScript collects the buffer."
- "If the runtime rejects external buffers, NAPI-RS falls back to copying the
  bytes into a runtime-owned buffer." (Electron-safe.)
- `Buffer` is listed under **Owned Types**: "can outlive the current native call
  and cross async boundaries" — safe as a return value.

## Risks

| Risk | Likelihood | Mitigation |
| ------ | ----------- | ------------ |
| Underlying `Vec<u8>` leaks | None | napi external-buffer finalizer drops the `Vec<u8>` on GC (documented). |
| Consumers doing `Buffer.from(img.buffer)` break | Very low | `Buffer.from(Buffer)` returns a copy; still works. Pre-change array consumers already wrapped. |
| Array access (`.length`, indexing) breaks | None | `Buffer` is indexable and has `.length`; identical access patterns. |
| Electron cannot zero-copy | Certain on Electron | napi falls back to copying bytes; correct, just not zero-copy. |
| `get_images()` output no longer `Array<number>` | Low | Intentional — spec requires `Buffer`; TS already declares `Buffer`. |
