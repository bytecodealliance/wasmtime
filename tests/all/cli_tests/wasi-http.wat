(module
  (import "wasi_snapshot_preview1" "proc_exit"
    (func $__wasi_proc_exit (param i32)))
  (import "wasi:io/streams" "write"
    (func $__wasi_io_streams_write (param i32 i32 i32 i32)))
  (import "wasi:io/streams" "blocking-write-and-flush"
    (func $__wasi_io_streams_blocking_write_and_flush (param i32 i32 i32 i32)))
  (import "wasi:io/streams" "subscribe-to-output-stream"
    (func $__wasi_io_streams_subscribe_to_output_stream (param i32) (result i32)))
  (import "wasi:http/types" "new-fields"
    (func $__wasi_http_types_new_fields (param i32 i32) (result i32)))
  (import "wasi:http/types" "drop-fields"
    (func $__wasi_http_types_drop_fields (param i32)))
  (import "wasi:http/types" "new-outgoing-request"
    (func $__wasi_http_types_new_outgoing_request (param i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32) (result i32)))
  (import "wasi:http/types" "outgoing-request-write"
    (func $__wasi_http_types_outgoing_request_write (param i32 i32)))
  (import "wasi:http/types" "drop-outgoing-request"
    (func $__wasi_http_types_drop_outgoing_request (param i32)))
  (func $_start
    (local i32)
    (local $headers_id i32)
    (local $request_id i32)
    (local $body_stream_id i32)

    ;; Print "Called _start".
    (call $print (i32.const 32) (i32.const 14))

    (local.set $headers_id (call $__wasi_http_types_new_fields
      i32.const 0 ;; base pointer
      i32.const 0 ;; length
    ))
    (local.set $request_id (call $__wasi_http_types_new_outgoing_request
      i32.const 0 ;; method = Method::Get
      i32.const 0 ;; method pointer
      i32.const 0 ;; method length
      i32.const 0 ;; path is some = None
      i32.const 0 ;; path pointer
      i32.const 0 ;; path length
      i32.const 1 ;; scheme is some = Some
      i32.const 1 ;; scheme = Scheme::Https
      i32.const 0 ;; scheme pointer
      i32.const 0 ;; scheme length
      i32.const 1 ;; authority is some = Some
      i32.const 96 ;; authority pointer = Constant value
      i32.const 15 ;; authority length
      local.get $headers_id ;; headers id
    ))
    (call $__wasi_http_types_outgoing_request_write (local.get $request_id) (local.get 0))
    local.get 0
    i32.const 4
    i32.add
    i32.load
    local.set $body_stream_id
    (call $__wasi_io_streams_write
      (local.get $body_stream_id) ;; body stream id (usually 8)
      (i32.const 128) ;; body stream pointer
      (i32.const 4) ;; body stream length
      (i32.const 0)
    )
    (drop (call $__wasi_io_streams_subscribe_to_output_stream (local.get $body_stream_id)))
    (call $__wasi_http_types_drop_fields (local.get $headers_id))
    (call $__wasi_http_types_drop_outgoing_request (local.get $request_id))

    (call $print (i32.const 64) (i32.const 5))
    (drop (call $__wasi_io_streams_subscribe_to_output_stream (i32.const 4)))

    (call $__wasi_proc_exit (i32.const 1))
  )

  ;; A helper function for printing ptr-len strings.
  (func $print (param $ptr i32) (param $len i32)
    (call $__wasi_io_streams_blocking_write_and_flush
      i32.const 4 ;; Value for stdout
      local.get $ptr
      local.get $len
      i32.const 0
    )
  )

  (func $cabi_realloc (param i32 i32 i32 i32) (result i32)
    i32.const 0
  )

  (memory 1)
  (export "memory" (memory 0))
  (export "_start" (func $_start))
  (export "cabi_realloc" (func $cabi_realloc))
  (data (i32.const 32) "Called _start\0a")
  (data (i32.const 64) "Done\0a")
  (data (i32.const 96) "www.example.com")
  (data (i32.const 128) "body")
)
