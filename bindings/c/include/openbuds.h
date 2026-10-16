/* SPDX-License-Identifier: GPL-3.0-or-later */
/* Copyright 2026 OpenBuds contributors */
#ifndef OPENBUDS_H
#define OPENBUDS_H
#include <stddef.h>
#include <stdint.h>
#if defined(_WIN32) && !defined(OPENBUDS_STATIC)
# ifdef OPENBUDS_BUILD
#  define OB_API __declspec(dllexport)
# else
#  define OB_API __declspec(dllimport)
# endif
#else
# define OB_API
#endif
#ifdef __cplusplus
extern "C" {
#endif
#define OB_ABI_V1 1u
#define OB_DRIVER_AIR7 "realme.buds-air7"
typedef uint64_t ob_handle_v1; /* 0 invalid; never a pointer */
typedef int32_t ob_status_v1;
enum {
 OB_OK=0, OB_INVALID_ARGUMENT=1, OB_UNKNOWN_DRIVER=2, OB_INVALID_HANDLE=3,
 OB_TIMEOUT=4, OB_IO=5, OB_DEVICE_REJECTED=6, OB_UNSUPPORTED=7, OB_BUSY=8,
 OB_CANCELLED=9, OB_NOT_INITIALIZED=10, OB_PROTOCOL=11, OB_INTERNAL=254, OB_PANIC=255
};
typedef struct ob_buffer_v1 { uint64_t id; const uint8_t *data; size_t len; } ob_buffer_v1;
typedef struct ob_error_info_v1 { int32_t status; uint32_t command; uint32_t device_status; uint32_t has_device_status; } ob_error_info_v1;
typedef struct ob_air7_noise_v1 { uint8_t action,kind,value,reserved; } ob_air7_noise_v1;
enum { OB_AIR7_OFF=1, OB_AIR7_TRANSPARENCY=2, OB_AIR7_MILD=4, OB_AIR7_MAXIMUM=8, OB_AIR7_MODERATE=16, OB_AIR7_ADAPTIVE=32 };
/* Callbacks return OB_OK, OB_TIMEOUT or an I/O failure. Counts must never exceed the slice.
 * read must honor timeout_ms; successful zero-byte read means EOF. Partial writes are valid.
 * Buffers are borrowed only for the callback. Never retain them or unwind/throw across C.
 * Calls are serialized per handle but may run on different caller threads. Context must
 * support this, or the application must keep all calls on its transport-owning thread.
 */
typedef ob_status_v1 (*ob_read_v1)(void *context,uint8_t *dst,size_t capacity,uint32_t timeout_ms,size_t *read_count);
typedef ob_status_v1 (*ob_write_v1)(void *context,const uint8_t *src,size_t length,size_t *write_count);
typedef ob_status_v1 (*ob_flush_v1)(void *context);
typedef void (*ob_release_v1)(void *context);
typedef struct ob_transport_v1 {
 uint32_t struct_size,abi_version;
 void *context;
 ob_read_v1 read;
 ob_write_v1 write;
 ob_flush_v1 flush; /* optional */
 ob_release_v1 release; /* optional; once, synchronously during successful close */
} ob_transport_v1;
/* Every pointer must be valid/aligned for its declared region. Null is accepted for an
 * input slice only when length=0. Outputs must not own an outstanding allocation.
 * UTF-8/JSON outputs are length-delimited, NOT NUL terminated. Free only with this DLL.
 * Callback-table validation failures do not transfer the context lease. After validation
 * and model acceptance, release is also called if subsequent construction fails.
 */
OB_API uint32_t ob_v1_abi_version(void);
OB_API ob_status_v1 ob_v1_driver_catalog(ob_buffer_v1 *out);
/* Remembered Windows devices, not an active inquiry. Contains private names/addresses.
 * connected is a host observation, not evidence of control-service ownership/model identity. */
OB_API ob_status_v1 ob_v1_windows_paired_devices(ob_buffer_v1 *out);
OB_API ob_status_v1 ob_v1_open(const uint8_t *model,size_t model_len,const ob_transport_v1 *transport,ob_handle_v1 *out);
OB_API ob_status_v1 ob_v1_open_windows(const uint8_t *model,size_t model_len,const uint8_t *address,size_t address_len,ob_handle_v1 *out);
OB_API ob_status_v1 ob_v1_initialize(ob_handle_v1 handle);
OB_API ob_status_v1 ob_v1_set_timeout(ob_handle_v1 handle,uint32_t milliseconds);
OB_API ob_status_v1 ob_v1_call(ob_handle_v1 handle,const uint8_t *request,size_t request_len,ob_buffer_v1 *out);
/* cancel is nonblocking; currently observed by bounded fit/finding helpers. Other calls
 * use the configured I/O timeout. Cancellation is not a hardware watchdog. */
OB_API ob_status_v1 ob_v1_cancel(ob_handle_v1 handle);
/* BUSY does not destroy the handle. Cancel/wait for in-flight calls, then retry close.
 * A successful close releases context before returning. Never close from its callback. */
OB_API ob_status_v1 ob_v1_close(ob_handle_v1 handle);
OB_API ob_status_v1 ob_v1_buffer_release(ob_buffer_v1 *buffer);
/* Error state is per thread; retrieve immediately after failure. Success clears the slot,
 * including last_error itself, which returns a copy. Release its message buffer. */
OB_API ob_status_v1 ob_v1_last_error(ob_error_info_v1 *info,ob_buffer_v1 *message);
OB_API ob_status_v1 ob_v1_air7_get_noise(ob_handle_v1 handle,ob_air7_noise_v1 *out);
OB_API ob_status_v1 ob_v1_air7_set_noise(ob_handle_v1 handle,uint32_t mode);
OB_API ob_status_v1 ob_v1_air7_set_feature(ob_handle_v1 handle,uint32_t feature,uint32_t enabled);
#ifdef __cplusplus
}
#endif
#endif
