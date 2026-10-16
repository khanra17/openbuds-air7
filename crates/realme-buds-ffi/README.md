# realme-buds-ffi

Versioned C ABI for the [OpenBuds SDK](../realme-buds/README.md). Builds `openbuds.dll`, `libopenbuds.so` or `libopenbuds.dylib`; no GUI dependency or embedded frontend.

The public contract is [include/openbuds.h](../../include/openbuds.h). See the [FFI guide](../../docs/ffi.md), [C/CMake example](../../examples/c/abi_smoke.c) and [Python ctypes example](../../examples/python/openbuds.py). Consumers can use prebuilt SDK archives without Rust.

The generic boundary handles lifetime, cancellation, callback streams and errors. JSON operations and direct POD convenience functions are model-scoped. Only `realme.buds-air7` is implemented; unknown models have no fallback. Native Windows RFCOMM is optional by platform; other hosts must supply callbacks.

```sh
cargo build -p realme-buds-ffi --release --locked
cargo test -p realme-buds-ffi --locked
```

GPL-3.0-or-later applies to this library and its FFI use. It is not an LGPL or permissively licensed exception. See [legal guidance](../../docs/legal.md).
