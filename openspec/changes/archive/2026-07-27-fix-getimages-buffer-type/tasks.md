## 1. Rust source: return real Buffer from `ToNapiValue`

- [x] 1.1 Edit `src/model/image.rs`: `NapiBuffer::to_napi_value` returns `Buffer::from(val.0)` via `<Buffer as ToNapiValue>::to_napi_value(env, buf)`
- [x] 1.2 Remove the "safe lie" comment above the `ToNapiValue` impl
- [x] 1.3 `cargo test` — 384 Rust tests pass

## 2. Add JS test for `getImages()` buffer runtime type

- [x] 2.1 Add test `getImages returns buffer as a real Buffer at runtime` — asserts `Buffer.isBuffer(true)` + `Buffer.compare` bytes match
- [x] 2.2 `pnpm test` — 136 tests pass (135 + 1 new)

## 3. Rebuild and verify

- [x] 3.1 `pnpm build` regenerates native artifacts
- [x] 3.2 `pnpm typecheck` — 0 errors
- [x] 3.3 Applied on `fix-addimage-buffer-type` branch (PR #38). CI pending push.
