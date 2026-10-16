# realme-buds-windows

Windows Bluetooth Classic transport adapter for [OpenBuds](../../README.md). Owns paired-device enumeration and RFCOMM socket I/O, not device commands or semantics.

`RfcommStream::connect_service(address, ServiceUuid)` requires an explicit service chosen by the device driver. The adapter does not assume an Air7 protocol, provide an Air7 default UUID or infer a driver from a friendly device name. It implements the SDK's timed reliable-stream contract.

Windows only. No audio-driver or Bluetooth-pairing UI is included. GPL-3.0-or-later.
