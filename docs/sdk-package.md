# OpenBuds native SDK

This archive contains a shared library and C header. **Using it does not require Rust or the desktop application.** Read `manifest.json` for its target architecture, source revision and ABI version.

- Windows: `bin/openbuds.dll`, `lib/openbuds.dll.lib` and `include/openbuds.h`.
- Linux/macOS: `lib/libopenbuds.so` or `lib/libopenbuds.dylib`, and `include/openbuds.h`.
- `examples/c`: a standalone CMake consumer with a synthetic callback stream.
- `examples/python`: a small standard-library-only ctypes consumer.

Use a binary matching your process architecture. Linux artifacts are built on Ubuntu 24.04; they are not a promise of compatibility with older glibc distributions. macOS builds are not signed/notarized application bundles. The Windows library uses native RFCOMM. Linux/macOS currently require application-provided transport callbacks; these builds do not add native Bluetooth backends.

## Try it without Bluetooth

From the extracted SDK directory:

```sh
cmake -S examples/c -B consumer-build -DOPENBUDS_SDK_DIR=/absolute/path/to/extracted-sdk
cmake --build consumer-build --config Release
ctest --test-dir consumer-build -C Release --output-on-failure
python examples/python/openbuds.py /absolute/path/to/shared-library
```

C compilation needs a C compiler/CMake, not Rust. Python only needs Python 3 and the shared library. These examples do not connect to hardware unless you explicitly supply `--address` to the Windows Python example.

## Contract

Select `realme.buds-air7` explicitly; unknown models are rejected, never treated as Air7. Open a stream session, initialize its driver, then call it. UTF-8 and JSON buffers are length-delimited, not NUL-terminated. Release every returned buffer with `ob_v1_buffer_release`, and every session with `ob_v1_close`, before unloading the library. Do not copy/free/use a buffer after release.

Calls are synchronous and serialized per session. Keep them off UI threads. Concurrent/reentrant operations return `OB_BUSY`. Cancellation signals bounded fit/finding helpers; an ordinary read still uses its I/O timeout. If close returns busy, cancel/wait for the active call and retry close. Callback context/code must survive until successful close; callbacks cannot throw/unwind or retain temporary buffers.

`include/openbuds.h` defines the ABI and callback ownership rules. The [online ABI guide](https://github.com/khanra17/openbuds-air7/blob/main/docs/ffi.md) describes the Air7 JSON schema and error handling; use the manifest's source revision for version-specific documentation. An acknowledged setter is not proof of stored readback or physical effect. Writes are not automatically retried after a timeout. Finding requires both earbuds physically removed; sensor reports are not a safety guarantee. Firmware writing is not exposed.

## Source and licensing

GPL-3.0-or-later applies to this SDK, including when linked through FFI. This is not a permissively licensed or LGPL bridge. Check the license obligations for your combined application before distributing it. Dependencies retain their own licenses; `THIRD_PARTY_LICENSES.html` is the workspace-wide report and may include dependencies not linked into this particular SDK.

The matching release includes a corresponding-source archive. `manifest.json` also identifies the exact public source revision. Product names identify compatibility only; no trademark rights, endorsement or legal immunity are granted.
