#include "proxy.h"


typedef struct {
  bool is_err;
  union {
    streams_tuple2_list_u8_bool_t ok;
  } val;
} streams_result_tuple2_list_u8_bool_stream_error_t;

typedef struct {
  bool is_err;
  union {
    streams_tuple2_u64_bool_t ok;
  } val;
} streams_result_tuple2_u64_bool_stream_error_t;

typedef struct {
  bool is_err;
  union {
    uint64_t ok;
  } val;
} streams_result_u64_stream_error_t;

typedef struct {
  bool is_some;
  types_trailers_t val;
} types_option_trailers_t;

typedef struct {
  bool is_some;
  types_scheme_t val;
} types_option_scheme_t;

typedef struct {
  bool is_err;
  union {
    types_incoming_stream_t ok;
  } val;
} types_result_incoming_stream_void_t;

typedef struct {
  bool is_err;
  union {
    types_outgoing_stream_t ok;
  } val;
} types_result_outgoing_stream_void_t;

typedef struct {
  bool is_err;
  union {
  } val;
} types_result_void_void_t;

typedef struct {
  bool is_some;
  types_result_incoming_response_error_t val;
} types_option_result_incoming_response_error_t;

typedef struct {
  bool is_some;
  default_outgoing_http_request_options_t val;
} default_outgoing_http_option_request_options_t;

__attribute__((import_module("random"), import_name("get-random-bytes")))
void __wasm_import_random_get_random_bytes(int64_t, int32_t);

__attribute__((import_module("random"), import_name("get-random-u64")))
int64_t __wasm_import_random_get_random_u64(void);

__attribute__((import_module("random"), import_name("insecure-random")))
void __wasm_import_random_insecure_random(int32_t);

__attribute__((import_module("console"), import_name("log")))
void __wasm_import_console_log(int32_t, int32_t, int32_t, int32_t, int32_t);

__attribute__((import_module("poll"), import_name("drop-pollable")))
void __wasm_import_poll_drop_pollable(int32_t);

__attribute__((import_module("poll"), import_name("poll-oneoff")))
void __wasm_import_poll_poll_oneoff(int32_t, int32_t, int32_t);

__attribute__((import_module("streams"), import_name("read")))
void __wasm_import_streams_read(int32_t, int64_t, int32_t);

__attribute__((import_module("streams"), import_name("skip")))
void __wasm_import_streams_skip(int32_t, int64_t, int32_t);

__attribute__((import_module("streams"), import_name("subscribe-to-input-stream")))
int32_t __wasm_import_streams_subscribe_to_input_stream(int32_t);

__attribute__((import_module("streams"), import_name("drop-input-stream")))
void __wasm_import_streams_drop_input_stream(int32_t);

__attribute__((import_module("streams"), import_name("write")))
void __wasm_import_streams_write(int32_t, int32_t, int32_t, int32_t);

__attribute__((import_module("streams"), import_name("write-zeroes")))
void __wasm_import_streams_write_zeroes(int32_t, int64_t, int32_t);

__attribute__((import_module("streams"), import_name("splice")))
void __wasm_import_streams_splice(int32_t, int32_t, int64_t, int32_t);

__attribute__((import_module("streams"), import_name("forward")))
void __wasm_import_streams_forward(int32_t, int32_t, int32_t);

__attribute__((import_module("streams"), import_name("subscribe-to-output-stream")))
int32_t __wasm_import_streams_subscribe_to_output_stream(int32_t);

__attribute__((import_module("streams"), import_name("drop-output-stream")))
void __wasm_import_streams_drop_output_stream(int32_t);

__attribute__((import_module("types"), import_name("drop-fields")))
void __wasm_import_types_drop_fields(int32_t);

__attribute__((import_module("types"), import_name("new-fields")))
int32_t __wasm_import_types_new_fields(int32_t, int32_t);

__attribute__((import_module("types"), import_name("fields-get")))
void __wasm_import_types_fields_get(int32_t, int32_t, int32_t, int32_t);

__attribute__((import_module("types"), import_name("fields-set")))
void __wasm_import_types_fields_set(int32_t, int32_t, int32_t, int32_t, int32_t);

__attribute__((import_module("types"), import_name("fields-delete")))
void __wasm_import_types_fields_delete(int32_t, int32_t, int32_t);

__attribute__((import_module("types"), import_name("fields-append")))
void __wasm_import_types_fields_append(int32_t, int32_t, int32_t, int32_t, int32_t);

__attribute__((import_module("types"), import_name("fields-entries")))
void __wasm_import_types_fields_entries(int32_t, int32_t);

__attribute__((import_module("types"), import_name("fields-clone")))
int32_t __wasm_import_types_fields_clone(int32_t);

__attribute__((import_module("types"), import_name("finish-incoming-stream")))
void __wasm_import_types_finish_incoming_stream(int32_t, int32_t);

__attribute__((import_module("types"), import_name("finish-outgoing-stream")))
void __wasm_import_types_finish_outgoing_stream(int32_t, int32_t, int32_t);

__attribute__((import_module("types"), import_name("drop-incoming-request")))
void __wasm_import_types_drop_incoming_request(int32_t);

__attribute__((import_module("types"), import_name("drop-outgoing-request")))
void __wasm_import_types_drop_outgoing_request(int32_t);

__attribute__((import_module("types"), import_name("incoming-request-method")))
void __wasm_import_types_incoming_request_method(int32_t, int32_t);

__attribute__((import_module("types"), import_name("incoming-request-path")))
void __wasm_import_types_incoming_request_path(int32_t, int32_t);

__attribute__((import_module("types"), import_name("incoming-request-query")))
void __wasm_import_types_incoming_request_query(int32_t, int32_t);

__attribute__((import_module("types"), import_name("incoming-request-scheme")))
void __wasm_import_types_incoming_request_scheme(int32_t, int32_t);

__attribute__((import_module("types"), import_name("incoming-request-authority")))
void __wasm_import_types_incoming_request_authority(int32_t, int32_t);

__attribute__((import_module("types"), import_name("incoming-request-headers")))
int32_t __wasm_import_types_incoming_request_headers(int32_t);

__attribute__((import_module("types"), import_name("incoming-request-consume")))
void __wasm_import_types_incoming_request_consume(int32_t, int32_t);

__attribute__((import_module("types"), import_name("new-outgoing-request")))
int32_t __wasm_import_types_new_outgoing_request(int32_t, int32_t, int32_t, int32_t, int32_t, int32_t, int32_t, int32_t, int32_t, int32_t, int32_t, int32_t, int32_t, int32_t);

__attribute__((import_module("types"), import_name("outgoing-request-write")))
void __wasm_import_types_outgoing_request_write(int32_t, int32_t);

__attribute__((import_module("types"), import_name("drop-response-outparam")))
void __wasm_import_types_drop_response_outparam(int32_t);

__attribute__((import_module("types"), import_name("set-response-outparam")))
int32_t __wasm_import_types_set_response_outparam(int32_t, int32_t, int32_t, int32_t);

__attribute__((import_module("types"), import_name("drop-incoming-response")))
void __wasm_import_types_drop_incoming_response(int32_t);

__attribute__((import_module("types"), import_name("drop-outgoing-response")))
void __wasm_import_types_drop_outgoing_response(int32_t);

__attribute__((import_module("types"), import_name("incoming-response-status")))
int32_t __wasm_import_types_incoming_response_status(int32_t);

__attribute__((import_module("types"), import_name("incoming-response-headers")))
int32_t __wasm_import_types_incoming_response_headers(int32_t);

__attribute__((import_module("types"), import_name("incoming-response-consume")))
void __wasm_import_types_incoming_response_consume(int32_t, int32_t);

__attribute__((import_module("types"), import_name("new-outgoing-response")))
int32_t __wasm_import_types_new_outgoing_response(int32_t, int32_t);

__attribute__((import_module("types"), import_name("outgoing-response-write")))
void __wasm_import_types_outgoing_response_write(int32_t, int32_t);

__attribute__((import_module("types"), import_name("drop-future-incoming-response")))
void __wasm_import_types_drop_future_incoming_response(int32_t);

__attribute__((import_module("types"), import_name("future-incoming-response-get")))
void __wasm_import_types_future_incoming_response_get(int32_t, int32_t);

__attribute__((import_module("types"), import_name("listen-to-future-incoming-response")))
int32_t __wasm_import_types_listen_to_future_incoming_response(int32_t);

__attribute__((import_module("default-outgoing-HTTP"), import_name("handle")))
int32_t __wasm_import_default_outgoing_http_handle(int32_t, int32_t, int32_t, int32_t, int32_t, int32_t, int32_t, int32_t);

__attribute__((weak, export_name("cabi_realloc")))
void *cabi_realloc(void *ptr, size_t old_size, size_t align, size_t new_size) {
  if (new_size == 0) return (void*) align;
  void *ret = realloc(ptr, new_size);
  if (!ret) abort();
  return ret;
}

// Helper Functions

void random_list_u8_free(random_list_u8_t *ptr) {
  if (ptr->len > 0) {
    free(ptr->ptr);
  }
}

void poll_list_pollable_free(poll_list_pollable_t *ptr) {
  if (ptr->len > 0) {
    free(ptr->ptr);
  }
}

void poll_list_u8_free(poll_list_u8_t *ptr) {
  if (ptr->len > 0) {
    free(ptr->ptr);
  }
}

void streams_list_u8_free(streams_list_u8_t *ptr) {
  if (ptr->len > 0) {
    free(ptr->ptr);
  }
}

void streams_tuple2_list_u8_bool_free(streams_tuple2_list_u8_bool_t *ptr) {
  streams_list_u8_free(&ptr->f0);
}

void types_scheme_free(types_scheme_t *ptr) {
  switch ((int32_t) ptr->tag) {
    case 2: {
      proxy_string_free(&ptr->val.other);
      break;
    }
  }
}

void types_method_free(types_method_t *ptr) {
  switch ((int32_t) ptr->tag) {
    case 9: {
      proxy_string_free(&ptr->val.other);
      break;
    }
  }
}

void types_error_free(types_error_t *ptr) {
  switch ((int32_t) ptr->tag) {
    case 0: {
      proxy_string_free(&ptr->val.invalid_url);
      break;
    }
    case 1: {
      proxy_string_free(&ptr->val.timeout_error);
      break;
    }
    case 2: {
      proxy_string_free(&ptr->val.protocol_error);
      break;
    }
    case 3: {
      proxy_string_free(&ptr->val.unexpected_error);
      break;
    }
  }
}

void types_tuple2_string_string_free(types_tuple2_string_string_t *ptr) {
  proxy_string_free(&ptr->f0);
  proxy_string_free(&ptr->f1);
}

void types_list_tuple2_string_string_free(types_list_tuple2_string_string_t *ptr) {
  for (size_t i = 0; i < ptr->len; i++) {
    types_tuple2_string_string_free(&ptr->ptr[i]);
  }
  if (ptr->len > 0) {
    free(ptr->ptr);
  }
}

void types_list_string_free(types_list_string_t *ptr) {
  for (size_t i = 0; i < ptr->len; i++) {
    proxy_string_free(&ptr->ptr[i]);
  }
  if (ptr->len > 0) {
    free(ptr->ptr);
  }
}

void types_result_outgoing_response_error_free(types_result_outgoing_response_error_t *ptr) {
  if (!ptr->is_err) {
  } else {
    types_error_free(&ptr->val.err);
  }
}

void types_result_incoming_response_error_free(types_result_incoming_response_error_t *ptr) {
  if (!ptr->is_err) {
  } else {
    types_error_free(&ptr->val.err);
  }
}

void proxy_string_set(proxy_string_t *ret, const char*s) {
  ret->ptr = (char*) s;
  ret->len = strlen(s);
}

void proxy_string_dup(proxy_string_t *ret, const char*s) {
  ret->len = strlen(s);
  ret->ptr = cabi_realloc(NULL, 0, 1, ret->len * 1);
  memcpy(ret->ptr, s, ret->len * 1);
}

void proxy_string_free(proxy_string_t *ret) {
  if (ret->len > 0) {
    free(ret->ptr);
  }
  ret->ptr = NULL;
  ret->len = 0;
}

// Component Adapters

void random_get_random_bytes(uint64_t len, random_list_u8_t *ret) {
  __attribute__((aligned(4)))
  uint8_t ret_area[8];
  int32_t ptr = (int32_t) &ret_area;
  __wasm_import_random_get_random_bytes((int64_t) (len), ptr);
  *ret = (random_list_u8_t) { (uint8_t*)(*((int32_t*) (ptr + 0))), (size_t)(*((int32_t*) (ptr + 4))) };
}

uint64_t random_get_random_u64(void) {
  int64_t ret = __wasm_import_random_get_random_u64();
  return (uint64_t) (ret);
}

void random_insecure_random(random_tuple2_u64_u64_t *ret) {
  __attribute__((aligned(8)))
  uint8_t ret_area[16];
  int32_t ptr = (int32_t) &ret_area;
  __wasm_import_random_insecure_random(ptr);
  *ret = (random_tuple2_u64_u64_t) {
    (uint64_t) (*((int64_t*) (ptr + 0))),
    (uint64_t) (*((int64_t*) (ptr + 8))),
  };
}

void console_log(console_level_t level, proxy_string_t *context, proxy_string_t *message) {
  __wasm_import_console_log((int32_t) level, (int32_t) (*context).ptr, (int32_t) (*context).len, (int32_t) (*message).ptr, (int32_t) (*message).len);
}

void poll_drop_pollable(poll_pollable_t this) {
  __wasm_import_poll_drop_pollable((int32_t) (this));
}

void poll_poll_oneoff(poll_list_pollable_t *in, poll_list_u8_t *ret) {
  __attribute__((aligned(4)))
  uint8_t ret_area[8];
  int32_t ptr = (int32_t) &ret_area;
  __wasm_import_poll_poll_oneoff((int32_t) (*in).ptr, (int32_t) (*in).len, ptr);
  *ret = (poll_list_u8_t) { (uint8_t*)(*((int32_t*) (ptr + 0))), (size_t)(*((int32_t*) (ptr + 4))) };
}

bool streams_read(streams_input_stream_t this, uint64_t len, streams_tuple2_list_u8_bool_t *ret, streams_stream_error_t *err) {
  __attribute__((aligned(4)))
  uint8_t ret_area[16];
  int32_t ptr = (int32_t) &ret_area;
  __wasm_import_streams_read((int32_t) (this), (int64_t) (len), ptr);
  streams_result_tuple2_list_u8_bool_stream_error_t result;
  switch ((int32_t) (*((uint8_t*) (ptr + 0)))) {
    case 0: {
      result.is_err = false;
      result.val.ok = (streams_tuple2_list_u8_bool_t) {
        (streams_list_u8_t) { (uint8_t*)(*((int32_t*) (ptr + 4))), (size_t)(*((int32_t*) (ptr + 8))) },
        (int32_t) (*((uint8_t*) (ptr + 12))),
      };
      break;
    }
    case 1: {
      result.is_err = true;
      break;
    }
  }
  if (!result.is_err) {
    *ret = result.val.ok;
    return 1;
  } else {
    return 0;
  }
}

bool streams_skip(streams_input_stream_t this, uint64_t len, streams_tuple2_u64_bool_t *ret, streams_stream_error_t *err) {
  __attribute__((aligned(8)))
  uint8_t ret_area[24];
  int32_t ptr = (int32_t) &ret_area;
  __wasm_import_streams_skip((int32_t) (this), (int64_t) (len), ptr);
  streams_result_tuple2_u64_bool_stream_error_t result;
  switch ((int32_t) (*((uint8_t*) (ptr + 0)))) {
    case 0: {
      result.is_err = false;
      result.val.ok = (streams_tuple2_u64_bool_t) {
        (uint64_t) (*((int64_t*) (ptr + 8))),
        (int32_t) (*((uint8_t*) (ptr + 16))),
      };
      break;
    }
    case 1: {
      result.is_err = true;
      break;
    }
  }
  if (!result.is_err) {
    *ret = result.val.ok;
    return 1;
  } else {
    return 0;
  }
}

streams_pollable_t streams_subscribe_to_input_stream(streams_input_stream_t this) {
  int32_t ret = __wasm_import_streams_subscribe_to_input_stream((int32_t) (this));
  return (uint32_t) (ret);
}

void streams_drop_input_stream(streams_input_stream_t this) {
  __wasm_import_streams_drop_input_stream((int32_t) (this));
}

bool streams_write(streams_output_stream_t this, streams_list_u8_t *buf, uint64_t *ret, streams_stream_error_t *err) {
  __attribute__((aligned(8)))
  uint8_t ret_area[16];
  int32_t ptr = (int32_t) &ret_area;
  __wasm_import_streams_write((int32_t) (this), (int32_t) (*buf).ptr, (int32_t) (*buf).len, ptr);
  streams_result_u64_stream_error_t result;
  switch ((int32_t) (*((uint8_t*) (ptr + 0)))) {
    case 0: {
      result.is_err = false;
      result.val.ok = (uint64_t) (*((int64_t*) (ptr + 8)));
      break;
    }
    case 1: {
      result.is_err = true;
      break;
    }
  }
  if (!result.is_err) {
    *ret = result.val.ok;
    return 1;
  } else {
    return 0;
  }
}

bool streams_write_zeroes(streams_output_stream_t this, uint64_t len, uint64_t *ret, streams_stream_error_t *err) {
  __attribute__((aligned(8)))
  uint8_t ret_area[16];
  int32_t ptr = (int32_t) &ret_area;
  __wasm_import_streams_write_zeroes((int32_t) (this), (int64_t) (len), ptr);
  streams_result_u64_stream_error_t result;
  switch ((int32_t) (*((uint8_t*) (ptr + 0)))) {
    case 0: {
      result.is_err = false;
      result.val.ok = (uint64_t) (*((int64_t*) (ptr + 8)));
      break;
    }
    case 1: {
      result.is_err = true;
      break;
    }
  }
  if (!result.is_err) {
    *ret = result.val.ok;
    return 1;
  } else {
    return 0;
  }
}

bool streams_splice(streams_output_stream_t this, streams_input_stream_t src, uint64_t len, streams_tuple2_u64_bool_t *ret, streams_stream_error_t *err) {
  __attribute__((aligned(8)))
  uint8_t ret_area[24];
  int32_t ptr = (int32_t) &ret_area;
  __wasm_import_streams_splice((int32_t) (this), (int32_t) (src), (int64_t) (len), ptr);
  streams_result_tuple2_u64_bool_stream_error_t result;
  switch ((int32_t) (*((uint8_t*) (ptr + 0)))) {
    case 0: {
      result.is_err = false;
      result.val.ok = (streams_tuple2_u64_bool_t) {
        (uint64_t) (*((int64_t*) (ptr + 8))),
        (int32_t) (*((uint8_t*) (ptr + 16))),
      };
      break;
    }
    case 1: {
      result.is_err = true;
      break;
    }
  }
  if (!result.is_err) {
    *ret = result.val.ok;
    return 1;
  } else {
    return 0;
  }
}

bool streams_forward(streams_output_stream_t this, streams_input_stream_t src, uint64_t *ret, streams_stream_error_t *err) {
  __attribute__((aligned(8)))
  uint8_t ret_area[16];
  int32_t ptr = (int32_t) &ret_area;
  __wasm_import_streams_forward((int32_t) (this), (int32_t) (src), ptr);
  streams_result_u64_stream_error_t result;
  switch ((int32_t) (*((uint8_t*) (ptr + 0)))) {
    case 0: {
      result.is_err = false;
      result.val.ok = (uint64_t) (*((int64_t*) (ptr + 8)));
      break;
    }
    case 1: {
      result.is_err = true;
      break;
    }
  }
  if (!result.is_err) {
    *ret = result.val.ok;
    return 1;
  } else {
    return 0;
  }
}

streams_pollable_t streams_subscribe_to_output_stream(streams_output_stream_t this) {
  int32_t ret = __wasm_import_streams_subscribe_to_output_stream((int32_t) (this));
  return (uint32_t) (ret);
}

void streams_drop_output_stream(streams_output_stream_t this) {
  __wasm_import_streams_drop_output_stream((int32_t) (this));
}

void types_drop_fields(types_fields_t fields) {
  __wasm_import_types_drop_fields((int32_t) (fields));
}

types_fields_t types_new_fields(types_list_tuple2_string_string_t *entries) {
  int32_t ret = __wasm_import_types_new_fields((int32_t) (*entries).ptr, (int32_t) (*entries).len);
  return (uint32_t) (ret);
}

void types_fields_get(types_fields_t fields, proxy_string_t *name, types_list_string_t *ret) {
  __attribute__((aligned(4)))
  uint8_t ret_area[8];
  int32_t ptr = (int32_t) &ret_area;
  __wasm_import_types_fields_get((int32_t) (fields), (int32_t) (*name).ptr, (int32_t) (*name).len, ptr);
  *ret = (types_list_string_t) { (proxy_string_t*)(*((int32_t*) (ptr + 0))), (size_t)(*((int32_t*) (ptr + 4))) };
}

void types_fields_set(types_fields_t fields, proxy_string_t *name, types_list_string_t *value) {
  __wasm_import_types_fields_set((int32_t) (fields), (int32_t) (*name).ptr, (int32_t) (*name).len, (int32_t) (*value).ptr, (int32_t) (*value).len);
}

void types_fields_delete(types_fields_t fields, proxy_string_t *name) {
  __wasm_import_types_fields_delete((int32_t) (fields), (int32_t) (*name).ptr, (int32_t) (*name).len);
}

void types_fields_append(types_fields_t fields, proxy_string_t *name, proxy_string_t *value) {
  __wasm_import_types_fields_append((int32_t) (fields), (int32_t) (*name).ptr, (int32_t) (*name).len, (int32_t) (*value).ptr, (int32_t) (*value).len);
}

void types_fields_entries(types_fields_t fields, types_list_tuple2_string_string_t *ret) {
  __attribute__((aligned(4)))
  uint8_t ret_area[8];
  int32_t ptr = (int32_t) &ret_area;
  __wasm_import_types_fields_entries((int32_t) (fields), ptr);
  *ret = (types_list_tuple2_string_string_t) { (types_tuple2_string_string_t*)(*((int32_t*) (ptr + 0))), (size_t)(*((int32_t*) (ptr + 4))) };
}

types_fields_t types_fields_clone(types_fields_t fields) {
  int32_t ret = __wasm_import_types_fields_clone((int32_t) (fields));
  return (uint32_t) (ret);
}

bool types_finish_incoming_stream(types_incoming_stream_t s, types_trailers_t *ret) {
  __attribute__((aligned(4)))
  uint8_t ret_area[8];
  int32_t ptr = (int32_t) &ret_area;
  __wasm_import_types_finish_incoming_stream((int32_t) (s), ptr);
  types_option_trailers_t option;
  switch ((int32_t) (*((uint8_t*) (ptr + 0)))) {
    case 0: {
      option.is_some = false;
      break;
    }
    case 1: {
      option.is_some = true;
      option.val = (uint32_t) (*((int32_t*) (ptr + 4)));
      break;
    }
  }
  *ret = option.val;
  return option.is_some;
}

void types_finish_outgoing_stream(types_outgoing_stream_t s, types_trailers_t *maybe_trailers) {
  types_option_trailers_t trailers;
  trailers.is_some = maybe_trailers != NULL;if (maybe_trailers) {
    trailers.val = *maybe_trailers;
  }
  int32_t option;
  int32_t option1;
  if ((trailers).is_some) {
    const types_trailers_t *payload0 = &(trailers).val;
    option = 1;
    option1 = (int32_t) (*payload0);
  } else {
    option = 0;
    option1 = 0;
  }
  __wasm_import_types_finish_outgoing_stream((int32_t) (s), option, option1);
}

void types_drop_incoming_request(types_incoming_request_t request) {
  __wasm_import_types_drop_incoming_request((int32_t) (request));
}

void types_drop_outgoing_request(types_outgoing_request_t request) {
  __wasm_import_types_drop_outgoing_request((int32_t) (request));
}

void types_incoming_request_method(types_incoming_request_t request, types_method_t *ret) {
  __attribute__((aligned(4)))
  uint8_t ret_area[12];
  int32_t ptr = (int32_t) &ret_area;
  __wasm_import_types_incoming_request_method((int32_t) (request), ptr);
  types_method_t variant;
  variant.tag = (int32_t) (*((uint8_t*) (ptr + 0)));
  switch ((int32_t) variant.tag) {
    case 0: {
      break;
    }
    case 1: {
      break;
    }
    case 2: {
      break;
    }
    case 3: {
      break;
    }
    case 4: {
      break;
    }
    case 5: {
      break;
    }
    case 6: {
      break;
    }
    case 7: {
      break;
    }
    case 8: {
      break;
    }
    case 9: {
      variant.val.other = (proxy_string_t) { (char*)(*((int32_t*) (ptr + 4))), (size_t)(*((int32_t*) (ptr + 8))) };
      break;
    }
  }
  *ret = variant;
}

void types_incoming_request_path(types_incoming_request_t request, proxy_string_t *ret) {
  __attribute__((aligned(4)))
  uint8_t ret_area[8];
  int32_t ptr = (int32_t) &ret_area;
  __wasm_import_types_incoming_request_path((int32_t) (request), ptr);
  *ret = (proxy_string_t) { (char*)(*((int32_t*) (ptr + 0))), (size_t)(*((int32_t*) (ptr + 4))) };
}

void types_incoming_request_query(types_incoming_request_t request, proxy_string_t *ret) {
  __attribute__((aligned(4)))
  uint8_t ret_area[8];
  int32_t ptr = (int32_t) &ret_area;
  __wasm_import_types_incoming_request_query((int32_t) (request), ptr);
  *ret = (proxy_string_t) { (char*)(*((int32_t*) (ptr + 0))), (size_t)(*((int32_t*) (ptr + 4))) };
}

bool types_incoming_request_scheme(types_incoming_request_t request, types_scheme_t *ret) {
  __attribute__((aligned(4)))
  uint8_t ret_area[16];
  int32_t ptr = (int32_t) &ret_area;
  __wasm_import_types_incoming_request_scheme((int32_t) (request), ptr);
  types_option_scheme_t option;
  switch ((int32_t) (*((uint8_t*) (ptr + 0)))) {
    case 0: {
      option.is_some = false;
      break;
    }
    case 1: {
      option.is_some = true;
      types_scheme_t variant;
      variant.tag = (int32_t) (*((uint8_t*) (ptr + 4)));
      switch ((int32_t) variant.tag) {
        case 0: {
          break;
        }
        case 1: {
          break;
        }
        case 2: {
          variant.val.other = (proxy_string_t) { (char*)(*((int32_t*) (ptr + 8))), (size_t)(*((int32_t*) (ptr + 12))) };
          break;
        }
      }
      
      option.val = variant;
      break;
    }
  }
  *ret = option.val;
  return option.is_some;
}

void types_incoming_request_authority(types_incoming_request_t request, proxy_string_t *ret) {
  __attribute__((aligned(4)))
  uint8_t ret_area[8];
  int32_t ptr = (int32_t) &ret_area;
  __wasm_import_types_incoming_request_authority((int32_t) (request), ptr);
  *ret = (proxy_string_t) { (char*)(*((int32_t*) (ptr + 0))), (size_t)(*((int32_t*) (ptr + 4))) };
}

types_headers_t types_incoming_request_headers(types_incoming_request_t request) {
  int32_t ret = __wasm_import_types_incoming_request_headers((int32_t) (request));
  return (uint32_t) (ret);
}

bool types_incoming_request_consume(types_incoming_request_t request, types_incoming_stream_t *ret) {
  __attribute__((aligned(4)))
  uint8_t ret_area[8];
  int32_t ptr = (int32_t) &ret_area;
  __wasm_import_types_incoming_request_consume((int32_t) (request), ptr);
  types_result_incoming_stream_void_t result;
  switch ((int32_t) (*((uint8_t*) (ptr + 0)))) {
    case 0: {
      result.is_err = false;
      result.val.ok = (uint32_t) (*((int32_t*) (ptr + 4)));
      break;
    }
    case 1: {
      result.is_err = true;
      break;
    }
  }
  if (!result.is_err) {
    *ret = result.val.ok;
    return 1;
  } else {
    return 0;
  }
}

types_outgoing_request_t types_new_outgoing_request(types_method_t *method, proxy_string_t *path, proxy_string_t *query, types_scheme_t *maybe_scheme, proxy_string_t *authority, types_headers_t headers) {
  types_option_scheme_t scheme;
  scheme.is_some = maybe_scheme != NULL;if (maybe_scheme) {
    scheme.val = *maybe_scheme;
  }
  int32_t variant;
  int32_t variant9;
  int32_t variant10;
  switch ((int32_t) (*method).tag) {
    case 0: {
      variant = 0;
      variant9 = 0;
      variant10 = 0;
      break;
    }
    case 1: {
      variant = 1;
      variant9 = 0;
      variant10 = 0;
      break;
    }
    case 2: {
      variant = 2;
      variant9 = 0;
      variant10 = 0;
      break;
    }
    case 3: {
      variant = 3;
      variant9 = 0;
      variant10 = 0;
      break;
    }
    case 4: {
      variant = 4;
      variant9 = 0;
      variant10 = 0;
      break;
    }
    case 5: {
      variant = 5;
      variant9 = 0;
      variant10 = 0;
      break;
    }
    case 6: {
      variant = 6;
      variant9 = 0;
      variant10 = 0;
      break;
    }
    case 7: {
      variant = 7;
      variant9 = 0;
      variant10 = 0;
      break;
    }
    case 8: {
      variant = 8;
      variant9 = 0;
      variant10 = 0;
      break;
    }
    case 9: {
      const proxy_string_t *payload8 = &(*method).val.other;
      variant = 9;
      variant9 = (int32_t) (*payload8).ptr;
      variant10 = (int32_t) (*payload8).len;
      break;
    }
  }
  int32_t option;
  int32_t option19;
  int32_t option20;
  int32_t option21;
  if ((scheme).is_some) {
    const types_scheme_t *payload12 = &(scheme).val;
    int32_t variant16;
    int32_t variant17;
    int32_t variant18;
    switch ((int32_t) (*payload12).tag) {
      case 0: {
        variant16 = 0;
        variant17 = 0;
        variant18 = 0;
        break;
      }
      case 1: {
        variant16 = 1;
        variant17 = 0;
        variant18 = 0;
        break;
      }
      case 2: {
        const proxy_string_t *payload15 = &(*payload12).val.other;
        variant16 = 2;
        variant17 = (int32_t) (*payload15).ptr;
        variant18 = (int32_t) (*payload15).len;
        break;
      }
    }
    option = 1;
    option19 = variant16;
    option20 = variant17;
    option21 = variant18;
  } else {
    option = 0;
    option19 = 0;
    option20 = 0;
    option21 = 0;
  }
  int32_t ret = __wasm_import_types_new_outgoing_request(variant, variant9, variant10, (int32_t) (*path).ptr, (int32_t) (*path).len, (int32_t) (*query).ptr, (int32_t) (*query).len, option, option19, option20, option21, (int32_t) (*authority).ptr, (int32_t) (*authority).len, (int32_t) (headers));
  return (uint32_t) (ret);
}

bool types_outgoing_request_write(types_outgoing_request_t request, types_outgoing_stream_t *ret) {
  __attribute__((aligned(4)))
  uint8_t ret_area[8];
  int32_t ptr = (int32_t) &ret_area;
  __wasm_import_types_outgoing_request_write((int32_t) (request), ptr);
  types_result_outgoing_stream_void_t result;
  switch ((int32_t) (*((uint8_t*) (ptr + 0)))) {
    case 0: {
      result.is_err = false;
      result.val.ok = (uint32_t) (*((int32_t*) (ptr + 4)));
      break;
    }
    case 1: {
      result.is_err = true;
      break;
    }
  }
  if (!result.is_err) {
    *ret = result.val.ok;
    return 1;
  } else {
    return 0;
  }
}

void types_drop_response_outparam(types_response_outparam_t response) {
  __wasm_import_types_drop_response_outparam((int32_t) (response));
}

bool types_set_response_outparam(types_result_outgoing_response_error_t *response) {
  int32_t result;
  int32_t result7;
  int32_t result8;
  int32_t result9;
  if ((*response).is_err) {
    const types_error_t *payload0 = &(*response).val.err;int32_t variant;
    int32_t variant5;
    int32_t variant6;
    switch ((int32_t) (*payload0).tag) {
      case 0: {
        const proxy_string_t *payload1 = &(*payload0).val.invalid_url;
        variant = 0;
        variant5 = (int32_t) (*payload1).ptr;
        variant6 = (int32_t) (*payload1).len;
        break;
      }
      case 1: {
        const proxy_string_t *payload2 = &(*payload0).val.timeout_error;
        variant = 1;
        variant5 = (int32_t) (*payload2).ptr;
        variant6 = (int32_t) (*payload2).len;
        break;
      }
      case 2: {
        const proxy_string_t *payload3 = &(*payload0).val.protocol_error;
        variant = 2;
        variant5 = (int32_t) (*payload3).ptr;
        variant6 = (int32_t) (*payload3).len;
        break;
      }
      case 3: {
        const proxy_string_t *payload4 = &(*payload0).val.unexpected_error;
        variant = 3;
        variant5 = (int32_t) (*payload4).ptr;
        variant6 = (int32_t) (*payload4).len;
        break;
      }
    }
    result = 1;
    result7 = variant;
    result8 = variant5;
    result9 = variant6;
  } else {
    const types_outgoing_response_t *payload = &(*response).val.ok;result = 0;
    result7 = (int32_t) (*payload);
    result8 = 0;
    result9 = 0;
  }
  int32_t ret = __wasm_import_types_set_response_outparam(result, result7, result8, result9);
  types_result_void_void_t result10;
  switch (ret) {
    case 0: {
      result10.is_err = false;
      break;
    }
    case 1: {
      result10.is_err = true;
      break;
    }
  }
  if (!result10.is_err) {
    return 1;
  } else {
    return 0;
  }
}

void types_drop_incoming_response(types_incoming_response_t response) {
  __wasm_import_types_drop_incoming_response((int32_t) (response));
}

void types_drop_outgoing_response(types_outgoing_response_t response) {
  __wasm_import_types_drop_outgoing_response((int32_t) (response));
}

types_status_code_t types_incoming_response_status(types_incoming_response_t response) {
  int32_t ret = __wasm_import_types_incoming_response_status((int32_t) (response));
  return (uint16_t) (ret);
}

types_headers_t types_incoming_response_headers(types_incoming_response_t response) {
  int32_t ret = __wasm_import_types_incoming_response_headers((int32_t) (response));
  return (uint32_t) (ret);
}

bool types_incoming_response_consume(types_incoming_response_t response, types_incoming_stream_t *ret) {
  __attribute__((aligned(4)))
  uint8_t ret_area[8];
  int32_t ptr = (int32_t) &ret_area;
  __wasm_import_types_incoming_response_consume((int32_t) (response), ptr);
  types_result_incoming_stream_void_t result;
  switch ((int32_t) (*((uint8_t*) (ptr + 0)))) {
    case 0: {
      result.is_err = false;
      result.val.ok = (uint32_t) (*((int32_t*) (ptr + 4)));
      break;
    }
    case 1: {
      result.is_err = true;
      break;
    }
  }
  if (!result.is_err) {
    *ret = result.val.ok;
    return 1;
  } else {
    return 0;
  }
}

types_outgoing_response_t types_new_outgoing_response(types_status_code_t status_code, types_headers_t headers) {
  int32_t ret = __wasm_import_types_new_outgoing_response((int32_t) (status_code), (int32_t) (headers));
  return (uint32_t) (ret);
}

bool types_outgoing_response_write(types_outgoing_response_t response, types_outgoing_stream_t *ret) {
  __attribute__((aligned(4)))
  uint8_t ret_area[8];
  int32_t ptr = (int32_t) &ret_area;
  __wasm_import_types_outgoing_response_write((int32_t) (response), ptr);
  types_result_outgoing_stream_void_t result;
  switch ((int32_t) (*((uint8_t*) (ptr + 0)))) {
    case 0: {
      result.is_err = false;
      result.val.ok = (uint32_t) (*((int32_t*) (ptr + 4)));
      break;
    }
    case 1: {
      result.is_err = true;
      break;
    }
  }
  if (!result.is_err) {
    *ret = result.val.ok;
    return 1;
  } else {
    return 0;
  }
}

void types_drop_future_incoming_response(types_future_incoming_response_t f) {
  __wasm_import_types_drop_future_incoming_response((int32_t) (f));
}

bool types_future_incoming_response_get(types_future_incoming_response_t f, types_result_incoming_response_error_t *ret) {
  __attribute__((aligned(4)))
  uint8_t ret_area[20];
  int32_t ptr = (int32_t) &ret_area;
  __wasm_import_types_future_incoming_response_get((int32_t) (f), ptr);
  types_option_result_incoming_response_error_t option;
  switch ((int32_t) (*((uint8_t*) (ptr + 0)))) {
    case 0: {
      option.is_some = false;
      break;
    }
    case 1: {
      option.is_some = true;
      types_result_incoming_response_error_t result;
      switch ((int32_t) (*((uint8_t*) (ptr + 4)))) {
        case 0: {
          result.is_err = false;
          result.val.ok = (uint32_t) (*((int32_t*) (ptr + 8)));
          break;
        }
        case 1: {
          result.is_err = true;
          types_error_t variant;
          variant.tag = (int32_t) (*((uint8_t*) (ptr + 8)));
          switch ((int32_t) variant.tag) {
            case 0: {
              variant.val.invalid_url = (proxy_string_t) { (char*)(*((int32_t*) (ptr + 12))), (size_t)(*((int32_t*) (ptr + 16))) };
              break;
            }
            case 1: {
              variant.val.timeout_error = (proxy_string_t) { (char*)(*((int32_t*) (ptr + 12))), (size_t)(*((int32_t*) (ptr + 16))) };
              break;
            }
            case 2: {
              variant.val.protocol_error = (proxy_string_t) { (char*)(*((int32_t*) (ptr + 12))), (size_t)(*((int32_t*) (ptr + 16))) };
              break;
            }
            case 3: {
              variant.val.unexpected_error = (proxy_string_t) { (char*)(*((int32_t*) (ptr + 12))), (size_t)(*((int32_t*) (ptr + 16))) };
              break;
            }
          }
          
          result.val.err = variant;
          break;
        }
      }
      
      option.val = result;
      break;
    }
  }
  *ret = option.val;
  return option.is_some;
}

types_pollable_t types_listen_to_future_incoming_response(types_future_incoming_response_t f) {
  int32_t ret = __wasm_import_types_listen_to_future_incoming_response((int32_t) (f));
  return (uint32_t) (ret);
}

default_outgoing_http_future_incoming_response_t default_outgoing_http_handle(default_outgoing_http_outgoing_request_t request, default_outgoing_http_request_options_t *maybe_options) {
  default_outgoing_http_option_request_options_t options;
  options.is_some = maybe_options != NULL;if (maybe_options) {
    options.val = *maybe_options;
  }
  int32_t option12;
  int32_t option13;
  int32_t option14;
  int32_t option15;
  int32_t option16;
  int32_t option17;
  int32_t option18;
  if ((options).is_some) {
    const default_outgoing_http_request_options_t *payload0 = &(options).val;
    int32_t option;
    int32_t option3;
    if (((*payload0).connect_timeout_ms).is_some) {
      const uint32_t *payload2 = &((*payload0).connect_timeout_ms).val;
      option = 1;
      option3 = (int32_t) (*payload2);
    } else {
      option = 0;
      option3 = 0;
    }
    int32_t option6;
    int32_t option7;
    if (((*payload0).first_byte_timeout_ms).is_some) {
      const uint32_t *payload5 = &((*payload0).first_byte_timeout_ms).val;
      option6 = 1;
      option7 = (int32_t) (*payload5);
    } else {
      option6 = 0;
      option7 = 0;
    }
    int32_t option10;
    int32_t option11;
    if (((*payload0).between_bytes_timeout_ms).is_some) {
      const uint32_t *payload9 = &((*payload0).between_bytes_timeout_ms).val;
      option10 = 1;
      option11 = (int32_t) (*payload9);
    } else {
      option10 = 0;
      option11 = 0;
    }
    option12 = 1;
    option13 = option;
    option14 = option3;
    option15 = option6;
    option16 = option7;
    option17 = option10;
    option18 = option11;
  } else {
    option12 = 0;
    option13 = 0;
    option14 = 0;
    option15 = 0;
    option16 = 0;
    option17 = 0;
    option18 = 0;
  }
  int32_t ret = __wasm_import_default_outgoing_http_handle((int32_t) (request), option12, option13, option14, option15, option16, option17, option18);
  return (uint32_t) (ret);
}

__attribute__((export_name("HTTP#handle")))
void __wasm_export_http_handle(int32_t arg, int32_t arg0) {
  http_handle((uint32_t) (arg), (uint32_t) (arg0));
}

extern void __component_type_object_force_link_proxy(void);
void __component_type_object_force_link_proxy_public_use_in_this_compilation_unit(void) {
  __component_type_object_force_link_proxy();
}
