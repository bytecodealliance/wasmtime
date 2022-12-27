#ifndef __BINDINGS_PROXY_H
#define __BINDINGS_PROXY_H
#ifdef __cplusplus
extern "C" {
#endif

#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <stdbool.h>

typedef struct {
  char*ptr;
  size_t len;
} proxy_string_t;

typedef struct {
  uint8_t *ptr;
  size_t len;
} random_list_u8_t;

typedef struct {
  uint64_t f0;
  uint64_t f1;
} random_tuple2_u64_u64_t;

// A log level, describing a kind of message.
typedef uint8_t console_level_t;

#define CONSOLE_LEVEL_TRACE 0
#define CONSOLE_LEVEL_DEBUG 1
#define CONSOLE_LEVEL_INFO 2
#define CONSOLE_LEVEL_WARN 3
#define CONSOLE_LEVEL_ERROR 4

// A "pollable" handle.
// 
// This is conceptually represents a `stream<_, _>`, or in other words,
// a stream that one can wait on, repeatedly, but which does not itself
// produce any data. It's temporary scaffolding until component-model's
// async features are ready.
// 
// And at present, it is a `u32` instead of being an actual handle, until
// the wit-bindgen implementation of handles and resources is ready.
// 
// `pollable` lifetimes are not automatically managed. Users must ensure
// that they do not outlive the resource they reference.
// 
// This [represents a resource](https://github.com/WebAssembly/WASI/blob/main/docs/WitInWasi.md#Resources).
typedef uint32_t poll_pollable_t;

typedef struct {
  poll_pollable_t *ptr;
  size_t len;
} poll_list_pollable_t;

typedef struct {
  uint8_t *ptr;
  size_t len;
} poll_list_u8_t;

typedef poll_pollable_t streams_pollable_t;

// An error type returned from a stream operation. Currently this
// doesn't provide any additional information.
typedef struct {
} streams_stream_error_t;

// An output bytestream. In the future, this will be replaced by handle
// types.
// 
// This conceptually represents a `stream<u8, _>`. It's temporary
// scaffolding until component-model's async features are ready.
// 
// `output-stream`s are *non-blocking* to the extent practical on
// underlying platforms. Except where specified otherwise, I/O operations also
// always return promptly, after the number of bytes that can be written
// promptly, which could even be zero. To wait for the stream to be ready to
// accept data, the `subscribe-to-output-stream` function to obtain a
// `pollable` which can be polled for using `wasi_poll`.
// 
// And at present, it is a `u32` instead of being an actual handle, until
// the wit-bindgen implementation of handles and resources is ready.
// 
// This [represents a resource](https://github.com/WebAssembly/WASI/blob/main/docs/WitInWasi.md#Resources).
typedef uint32_t streams_output_stream_t;

// An input bytestream. In the future, this will be replaced by handle
// types.
// 
// This conceptually represents a `stream<u8, _>`. It's temporary
// scaffolding until component-model's async features are ready.
// 
// `input-stream`s are *non-blocking* to the extent practical on underlying
// platforms. I/O operations always return promptly; if fewer bytes are
// promptly available than requested, they return the number of bytes promptly
// available, which could even be zero. To wait for data to be available,
// use the `subscribe-to-input-stream` function to obtain a `pollable` which
// can be polled for using `wasi_poll`.
// 
// And at present, it is a `u32` instead of being an actual handle, until
// the wit-bindgen implementation of handles and resources is ready.
// 
// This [represents a resource](https://github.com/WebAssembly/WASI/blob/main/docs/WitInWasi.md#Resources).
typedef uint32_t streams_input_stream_t;

typedef struct {
  uint8_t *ptr;
  size_t len;
} streams_list_u8_t;

typedef struct {
  streams_list_u8_t f0;
  bool f1;
} streams_tuple2_list_u8_bool_t;

typedef struct {
  uint64_t f0;
  bool f1;
} streams_tuple2_u64_bool_t;

typedef streams_input_stream_t types_input_stream_t;

typedef streams_output_stream_t types_output_stream_t;

typedef poll_pollable_t types_pollable_t;

typedef uint16_t types_status_code_t;

typedef struct {
  uint8_t tag;
  union {
    proxy_string_t other;
  } val;
} types_scheme_t;

#define TYPES_SCHEME_HTTP 0
#define TYPES_SCHEME_HTTPS 1
#define TYPES_SCHEME_OTHER 2

typedef uint32_t types_response_outparam_t;

typedef struct {
  bool is_some;
  uint32_t val;
} types_option_u32_t;

typedef struct {
  types_option_u32_t connect_timeout_ms;
  types_option_u32_t first_byte_timeout_ms;
  types_option_u32_t between_bytes_timeout_ms;
} types_request_options_t;

typedef types_output_stream_t types_outgoing_stream_t;

typedef uint32_t types_outgoing_response_t;

typedef uint32_t types_outgoing_request_t;

typedef struct {
  uint8_t tag;
  union {
    proxy_string_t other;
  } val;
} types_method_t;

#define TYPES_METHOD_GET 0
#define TYPES_METHOD_HEAD 1
#define TYPES_METHOD_POST 2
#define TYPES_METHOD_PUT 3
#define TYPES_METHOD_DELETE 4
#define TYPES_METHOD_CONNECT 5
#define TYPES_METHOD_OPTIONS 6
#define TYPES_METHOD_TRACE 7
#define TYPES_METHOD_PATCH 8
#define TYPES_METHOD_OTHER 9

typedef types_input_stream_t types_incoming_stream_t;

typedef uint32_t types_incoming_response_t;

typedef uint32_t types_incoming_request_t;

typedef uint32_t types_future_incoming_response_t;

typedef uint32_t types_fields_t;

typedef types_fields_t types_trailers_t;

typedef types_fields_t types_headers_t;

typedef struct {
  uint8_t tag;
  union {
    proxy_string_t invalid_url;
    proxy_string_t timeout_error;
    proxy_string_t protocol_error;
    proxy_string_t unexpected_error;
  } val;
} types_error_t;

#define TYPES_ERROR_INVALID_URL 0
#define TYPES_ERROR_TIMEOUT_ERROR 1
#define TYPES_ERROR_PROTOCOL_ERROR 2
#define TYPES_ERROR_UNEXPECTED_ERROR 3

typedef struct {
  proxy_string_t f0;
  proxy_string_t f1;
} types_tuple2_string_string_t;

typedef struct {
  types_tuple2_string_string_t *ptr;
  size_t len;
} types_list_tuple2_string_string_t;

typedef struct {
  proxy_string_t *ptr;
  size_t len;
} types_list_string_t;

typedef struct {
  bool is_err;
  union {
    types_outgoing_response_t ok;
    types_error_t err;
  } val;
} types_result_outgoing_response_error_t;

typedef struct {
  bool is_err;
  union {
    types_incoming_response_t ok;
    types_error_t err;
  } val;
} types_result_incoming_response_error_t;

typedef types_outgoing_request_t default_outgoing_http_outgoing_request_t;

typedef types_request_options_t default_outgoing_http_request_options_t;

typedef types_future_incoming_response_t default_outgoing_http_future_incoming_response_t;

typedef types_incoming_request_t http_incoming_request_t;

typedef types_response_outparam_t http_response_outparam_t;

// Imported Functions from `random`
void random_get_random_bytes(uint64_t len, random_list_u8_t *ret);
uint64_t random_get_random_u64(void);
void random_insecure_random(random_tuple2_u64_u64_t *ret);

// Imported Functions from `console`
void console_log(console_level_t level, proxy_string_t *context, proxy_string_t *message);

// Imported Functions from `poll`
void poll_drop_pollable(poll_pollable_t this);
void poll_poll_oneoff(poll_list_pollable_t *in, poll_list_u8_t *ret);

// Imported Functions from `streams`
bool streams_read(streams_input_stream_t this, uint64_t len, streams_tuple2_list_u8_bool_t *ret, streams_stream_error_t *err);
bool streams_skip(streams_input_stream_t this, uint64_t len, streams_tuple2_u64_bool_t *ret, streams_stream_error_t *err);
streams_pollable_t streams_subscribe_to_input_stream(streams_input_stream_t this);
void streams_drop_input_stream(streams_input_stream_t this);
bool streams_write(streams_output_stream_t this, streams_list_u8_t *buf, uint64_t *ret, streams_stream_error_t *err);
bool streams_write_zeroes(streams_output_stream_t this, uint64_t len, uint64_t *ret, streams_stream_error_t *err);
bool streams_splice(streams_output_stream_t this, streams_input_stream_t src, uint64_t len, streams_tuple2_u64_bool_t *ret, streams_stream_error_t *err);
bool streams_forward(streams_output_stream_t this, streams_input_stream_t src, uint64_t *ret, streams_stream_error_t *err);
streams_pollable_t streams_subscribe_to_output_stream(streams_output_stream_t this);
void streams_drop_output_stream(streams_output_stream_t this);

// Imported Functions from `types`
void types_drop_fields(types_fields_t fields);
types_fields_t types_new_fields(types_list_tuple2_string_string_t *entries);
void types_fields_get(types_fields_t fields, proxy_string_t *name, types_list_string_t *ret);
void types_fields_set(types_fields_t fields, proxy_string_t *name, types_list_string_t *value);
void types_fields_delete(types_fields_t fields, proxy_string_t *name);
void types_fields_append(types_fields_t fields, proxy_string_t *name, proxy_string_t *value);
void types_fields_entries(types_fields_t fields, types_list_tuple2_string_string_t *ret);
types_fields_t types_fields_clone(types_fields_t fields);
bool types_finish_incoming_stream(types_incoming_stream_t s, types_trailers_t *ret);
void types_finish_outgoing_stream(types_outgoing_stream_t s, types_trailers_t *maybe_trailers);
void types_drop_incoming_request(types_incoming_request_t request);
void types_drop_outgoing_request(types_outgoing_request_t request);
void types_incoming_request_method(types_incoming_request_t request, types_method_t *ret);
void types_incoming_request_path(types_incoming_request_t request, proxy_string_t *ret);
void types_incoming_request_query(types_incoming_request_t request, proxy_string_t *ret);
bool types_incoming_request_scheme(types_incoming_request_t request, types_scheme_t *ret);
void types_incoming_request_authority(types_incoming_request_t request, proxy_string_t *ret);
types_headers_t types_incoming_request_headers(types_incoming_request_t request);
bool types_incoming_request_consume(types_incoming_request_t request, types_incoming_stream_t *ret);
types_outgoing_request_t types_new_outgoing_request(types_method_t *method, proxy_string_t *path, proxy_string_t *query, types_scheme_t *maybe_scheme, proxy_string_t *authority, types_headers_t headers);
bool types_outgoing_request_write(types_outgoing_request_t request, types_outgoing_stream_t *ret);
void types_drop_response_outparam(types_response_outparam_t response);
bool types_set_response_outparam(types_result_outgoing_response_error_t *response);
void types_drop_incoming_response(types_incoming_response_t response);
void types_drop_outgoing_response(types_outgoing_response_t response);
types_status_code_t types_incoming_response_status(types_incoming_response_t response);
types_headers_t types_incoming_response_headers(types_incoming_response_t response);
bool types_incoming_response_consume(types_incoming_response_t response, types_incoming_stream_t *ret);
types_outgoing_response_t types_new_outgoing_response(types_status_code_t status_code, types_headers_t headers);
bool types_outgoing_response_write(types_outgoing_response_t response, types_outgoing_stream_t *ret);
void types_drop_future_incoming_response(types_future_incoming_response_t f);
bool types_future_incoming_response_get(types_future_incoming_response_t f, types_result_incoming_response_error_t *ret);
types_pollable_t types_listen_to_future_incoming_response(types_future_incoming_response_t f);

// Imported Functions from `default-outgoing-HTTP`
default_outgoing_http_future_incoming_response_t default_outgoing_http_handle(default_outgoing_http_outgoing_request_t request, default_outgoing_http_request_options_t *maybe_options);

// Exported Functions from `HTTP`
void http_handle(http_incoming_request_t request, http_response_outparam_t response_out);

// Helper Functions

void random_list_u8_free(random_list_u8_t *ptr);
void poll_list_pollable_free(poll_list_pollable_t *ptr);
void poll_list_u8_free(poll_list_u8_t *ptr);
void streams_list_u8_free(streams_list_u8_t *ptr);
void streams_tuple2_list_u8_bool_free(streams_tuple2_list_u8_bool_t *ptr);
void types_scheme_free(types_scheme_t *ptr);
void types_method_free(types_method_t *ptr);
void types_error_free(types_error_t *ptr);
void types_tuple2_string_string_free(types_tuple2_string_string_t *ptr);
void types_list_tuple2_string_string_free(types_list_tuple2_string_string_t *ptr);
void types_list_string_free(types_list_string_t *ptr);
void types_result_outgoing_response_error_free(types_result_outgoing_response_error_t *ptr);
void types_result_incoming_response_error_free(types_result_incoming_response_error_t *ptr);
void proxy_string_set(proxy_string_t *ret, const char*s);
void proxy_string_dup(proxy_string_t *ret, const char*s);
void proxy_string_free(proxy_string_t *ret);

#ifdef __cplusplus
}
#endif
#endif
