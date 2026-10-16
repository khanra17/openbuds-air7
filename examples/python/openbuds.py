# SPDX-License-Identifier: GPL-3.0-or-later
"""Small ctypes consumer of the prebuilt SDK; Python needs no Rust installation."""
import argparse
import ctypes as c
import json
from pathlib import Path


class Buffer(c.Structure):
    _fields_ = [("id", c.c_uint64), ("data", c.POINTER(c.c_uint8)), ("len", c.c_size_t)]


class ErrorInfo(c.Structure):
    _fields_ = [("status", c.c_int32), ("command", c.c_uint32),
                ("device_status", c.c_uint32), ("has_device_status", c.c_uint32)]


class OpenBuds:
    def __init__(self, library):
        self.dll = c.CDLL(str(Path(library).resolve()))
        signatures = {
            "abi_version": (c.c_uint32, []),
            "driver_catalog": (c.c_int32, [c.POINTER(Buffer)]),
            "open_windows": (c.c_int32, [c.c_char_p, c.c_size_t, c.c_char_p, c.c_size_t, c.POINTER(c.c_uint64)]),
            "initialize": (c.c_int32, [c.c_uint64]),
            "call": (c.c_int32, [c.c_uint64, c.c_char_p, c.c_size_t, c.POINTER(Buffer)]),
            "cancel": (c.c_int32, [c.c_uint64]),
            "close": (c.c_int32, [c.c_uint64]),
            "buffer_release": (c.c_int32, [c.POINTER(Buffer)]),
            "last_error": (c.c_int32, [c.POINTER(ErrorInfo), c.POINTER(Buffer)]),
        }
        for name, (result, args) in signatures.items():
            fn = getattr(self.dll, "ob_v1_" + name)
            fn.restype, fn.argtypes = result, args
            setattr(self, "_" + name, fn)
        if self._abi_version() != 1:
            raise RuntimeError("This example requires OpenBuds ABI v1")

    def _check(self, status):
        if status:
            info, message = ErrorInfo(), Buffer()
            if self._last_error(c.byref(info), c.byref(message)):
                raise RuntimeError(f"OpenBuds error {status} (error retrieval failed)")
            try:
                text = c.string_at(message.data, message.len).decode("utf-8")
            finally:
                self._buffer_release(c.byref(message))
            detail = f"; command={info.command:#x}, device status={info.device_status}" if info.has_device_status else ""
            raise RuntimeError(f"OpenBuds error {status}: {text}{detail}")

    def _json(self, buffer):
        try:
            return json.loads(c.string_at(buffer.data, buffer.len))
        finally:
            self._check(self._buffer_release(c.byref(buffer)))

    def catalog(self):
        buffer = Buffer()
        self._check(self._driver_catalog(c.byref(buffer)))
        return self._json(buffer)

    def connect_windows(self, address, model="realme.buds-air7"):
        model, address = model.encode("utf-8"), address.encode("utf-8")
        handle = c.c_uint64()
        self._check(self._open_windows(model, len(model), address, len(address), c.byref(handle)))
        device = Device(self, handle.value)
        try:
            self._check(self._initialize(device.handle))
        except BaseException:
            device.close()
            raise
        return device


class Device:
    def __init__(self, sdk, handle):
        self.sdk, self.handle = sdk, handle

    def call(self, request):
        data = json.dumps(request, ensure_ascii=False).encode("utf-8")
        buffer = Buffer()
        self.sdk._check(self.sdk._call(self.handle, data, len(data), c.byref(buffer)))
        return self.sdk._json(buffer)

    def cancel(self):
        self.sdk._check(self.sdk._cancel(self.handle))

    def close(self):
        if self.handle:
            self.sdk._check(self.sdk._close(self.handle))
            self.handle = 0

    def __enter__(self):
        return self

    def __exit__(self, *_):
        self.close()


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("library", help="Path to openbuds.dll, libopenbuds.so or libopenbuds.dylib")
    parser.add_argument("--address", help="Opt in to native Windows Bluetooth; otherwise only print the offline driver catalog")
    args = parser.parse_args()
    sdk = OpenBuds(args.library)
    if args.address:
        with sdk.connect_windows(args.address) as buds:
            print(json.dumps(buds.call({"op": "snapshot"}), indent=2))
    else:
        print(json.dumps(sdk.catalog(), indent=2))
