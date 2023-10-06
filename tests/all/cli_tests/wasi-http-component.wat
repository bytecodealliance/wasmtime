(component
  (import (interface "wasi:io/streams") (instance $wasi_io_streams
    (export (;0;) "output-stream" (type (sub resource)))
    (export (;1;) "error" (type (sub resource)))
    (type (;2;) (own 1))
    (type (;3;) (variant (case "last-operation-failed" 2) (case "closed")))
    (export (;4;) "stream-error" (type (eq 3)))
    (type (;5;) (borrow 0))
    (type (;6;) (list u8))
    (type (;7;) (result (error 4)))
    (type (;8;) (func (param "self" 5) (param "contents" 6) (result 7)))
    (export (;0;) "[method]output-stream.blocking-write-and-flush" (func (type 8)))
  ))
  (alias export $wasi_io_streams "output-stream" (type $resource_output_stream))

  (import (interface "wasi:cli/stdout") (instance $wasi_cli_stdout
    (alias outer 1 $resource_output_stream (type (;0;)))
    (export (;1;) "output-stream" (type (eq 0)))
    (type (;2;) (own 1))
    (type (;3;) (func (result 2)))
    (export (;0;) "get-stdout" (func (type 3)))
  ))

  (import (interface "wasi:http/types") (instance $wasi_http_types
    (export (;0;) "fields" (type (sub resource)))
    (type (;1;) (variant (case "get") (case "head") (case "post") (case "put") (case "delete") (case "connect") (case "options") (case "trace") (case "patch") (case "other" string)))
    (export (;2;) "method" (type (eq 1)))
    (type (;3;) (variant (case "HTTP") (case "HTTPS") (case "other" string)))
    (export (;4;) "scheme" (type (eq 3)))
    (export (;5;) "headers" (type (eq 0)))
    (export (;6;) "outgoing-request" (type (sub resource)))
    (export (;7;) "outgoing-body" (type (sub resource)))
    (type (;8;) u16)
    (export (;9;) "status-code" (type (eq 8)))
    (export (;10;) "outgoing-response" (type (sub resource)))
    (alias outer 1 $resource_output_stream (type (;11;)))
    (export (;12;) "output-stream" (type (eq 11)))
    (type (;13;) (list u8))
    (type (;14;) (tuple string 13))
    (type (;15;) (list 14))
    (type (;16;) (own 0))
    (type (;17;) (func (param "entries" 15) (result 16)))
    (export (;0;) "[constructor]fields" (func (type 17)))
    (type (;18;) (option string))
    (type (;19;) (option 4))
    (type (;20;) (own 5))
    (type (;21;) (own 6))
    (type (;22;) (func (param "method" 2) (param "path-with-query" 18) (param "scheme" 19) (param "authority" 18) (param "headers" 20) (result 21)))
    (export (;1;) "[constructor]outgoing-request" (func (type 22)))
    (type (;23;) (borrow 6))
    (type (;24;) (own 7))
    (type (;25;) (result 24))
    (type (;26;) (func (param "self" 23) (result 25)))
    (export (;2;) "[method]outgoing-request.write" (func (type 26)))
    (type (;27;) (own 10))
    (type (;28;) (func (param "status-code" 9) (param "headers" 20) (result 27)))
    (export (;3;) "[constructor]outgoing-response" (func (type 28)))
    (type (;29;) (borrow 10))
    (type (;30;) (func (param "self" 29) (result 25)))
    (export (;4;) "[method]outgoing-response.write" (func (type 30)))
    (type (;31;) (borrow 7))
    (type (;32;) (own 12))
    (type (;33;) (result 32))
    (type (;34;) (func (param "self" 31) (result 33)))
    (export (;5;) "[method]outgoing-body.write" (func (type 34)))
  ))
  (alias export $wasi_http_types "outgoing-body" (type $resource_outgoing_body))
  (alias export $wasi_http_types "outgoing-request" (type $resource_outgoing_request))
  (alias export $wasi_http_types "outgoing-response" (type $resource_outgoing_response))

  (core module $m
    (import "wasi:cli/stdout" "get-stdout"
      (func $__wasi_cli_stdout_getstdout (result i32)))

    (import "wasi:io/streams" "[method]output-stream.blocking-write-and-flush"
      (func $__wasi_io_streams_method_outputstream_blockingwriteandflush (param i32 i32 i32 i32)))
    (import "wasi:io/streams" "[resource-drop]output-stream"
      (func $__wasi_io_streams_resourcedrop_outputstream (param i32)))

    (import "wasi:http/types" "[constructor]fields"
      (func $__wasi_http_types_constructor_fields (param i32 i32) (result i32)))
    (import "wasi:http/types" "[constructor]outgoing-request"
      (func $__wasi_http_types_constructor_outgoingrequest (param i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32) (result i32)))
    (import "wasi:http/types" "[constructor]outgoing-response"
      (func $__wasi_http_types_constructor_outgoingresponse (param i32 i32) (result i32)))
    (import "wasi:http/types" "[method]outgoing-body.write"
      (func $__wasi_http_types_method_outgoingbody_write (param i32 i32)))
    (import "wasi:http/types" "[method]outgoing-request.write"
      (func $__wasi_http_types_method_outgoingrequest_write (param i32 i32)))
    (import "wasi:http/types" "[method]outgoing-response.write"
      (func $__wasi_http_types_method_outgoingresponse_write (param i32 i32)))
    (import "wasi:http/types" "[resource-drop]outgoing-body"
      (func $__wasi_http_types_resourcedrop_outgoingbody (param i32)))
    (import "wasi:http/types" "[resource-drop]outgoing-request"
      (func $__wasi_http_types_resourcedrop_outgoingrequest (param i32)))
    (import "wasi:http/types" "[resource-drop]outgoing-response"
      (func $__wasi_http_types_resourcedrop_outgoingresponse (param i32)))

    (func $_start (result i32)
      (local i32 i32)
      (local $headers_handle i32)
      (local $request_handle i32)
      (local $response_handle i32)
      (local $body_stream_handle i32)

      ;; Print "Called _start" to standard output.
      (call $print (i32.const 32) (i32.const 14))

      (local.set $headers_handle (call $__wasi_http_types_constructor_fields
        i32.const 0 ;; base pointer
        i32.const 0 ;; length
      ))
      (local.set $request_handle (call $__wasi_http_types_constructor_outgoingrequest
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
        local.get $headers_handle ;; headers resource handle
      ))
      (call $__wasi_http_types_method_outgoingrequest_write (local.get $request_handle) (local.get 0))
      (local.set 1 (i32.load (call $unwrap_result (local.get 0))))
      (call $__wasi_http_types_method_outgoingbody_write (local.get 1) (local.get 0))
      (local.set $body_stream_handle (i32.load (call $unwrap_result (local.get 0))))
      (call $__wasi_io_streams_method_outputstream_blockingwriteandflush
        (local.get $body_stream_handle) ;; resource handle for request body stream
        (i32.const 128) ;; body content pointer
        (i32.const 4) ;; body content length
        (i32.const 0)
      )
      (call $__wasi_io_streams_resourcedrop_outputstream (local.get $body_stream_handle))
      (call $__wasi_http_types_resourcedrop_outgoingrequest (local.get $request_handle))

      (local.set $headers_handle (call $__wasi_http_types_constructor_fields
        i32.const 0 ;; base pointer
        i32.const 0 ;; length
      ))
      (local.set $response_handle (call $__wasi_http_types_constructor_outgoingresponse
        i32.const 200 ;; status code
        local.get $headers_handle ;; headers resource handle
      ))
      (call $__wasi_http_types_method_outgoingresponse_write (local.get $response_handle) (local.get 0))
      (local.set 1 (i32.load (call $unwrap_result (local.get 0))))
      (call $__wasi_http_types_method_outgoingbody_write (local.get 1) (local.get 0))
      (local.set $body_stream_handle (i32.load (call $unwrap_result (local.get 0))))
      (call $__wasi_io_streams_method_outputstream_blockingwriteandflush
        (local.get $body_stream_handle) ;; resource handle for response body stream
        (i32.const 128) ;; body content pointer
        (i32.const 4) ;; body content length
        (i32.const 0)
      )
      (call $__wasi_io_streams_resourcedrop_outputstream (local.get $body_stream_handle))
      (call $__wasi_http_types_resourcedrop_outgoingresponse (local.get $response_handle))

      ;; Print "Done" to standard output.
      (call $print (i32.const 64) (i32.const 5))

      i32.const 0
    )

    ;; A helper function for printing a slice of bytes.
    (func $print (param $ptr i32) (param $len i32)
      (local i32 i32)
      (local $stdout i32)
      (local.set $stdout (call $__wasi_cli_stdout_getstdout))

      (call $__wasi_io_streams_method_outputstream_blockingwriteandflush
        local.get $stdout
        local.get $ptr
        local.get $len
        local.get 0
      )

      (call $__wasi_io_streams_resourcedrop_outputstream (local.get $stdout))
    )

    ;; A helper function for unwrapping result type
    (func $unwrap_result (param $ptr i32) (result i32)
      (if (i32.ne (i32.load (local.get $ptr)) (i32.const 0))
        (then
          unreachable
        )
      )
      local.get $ptr
      i32.const 4
      return i32.add
    )

    (func $cabi_realloc (param i32 i32 i32 i32) (result i32)
      i32.const 0
    )

    (memory 1)
    (export "memory" (memory 0))
    (export "run" (func $_start))
    (export "cabi_realloc" (func $cabi_realloc))
    (data (i32.const 32) "Called _start\0a")
    (data (i32.const 64) "Done\0a")
    (data (i32.const 96) "www.example.com")
    (data (i32.const 128) "body")
  )
  (core module $indirect-module
    (type (;0;) (func (param i32 i32 i32 i32)))
    (type (;1;) (func (param i32 i32) (result i32)))
    (type (;2;) (func (param i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32) (result i32)))
    (type (;3;) (func (param i32 i32)))
    (func $#func0<indirect-wasi:io/streams-_method_output-stream.blocking-write-and-flush> (@name "indirect-wasi:io/streams-[method]output-stream.blocking-write-and-flush") (;0;) (type 0) (param i32 i32 i32 i32)
      local.get 0
      local.get 1
      local.get 2
      local.get 3
      i32.const 0
      call_indirect (type 0)
    )
    (func $#func1<indirect-wasi:http/types-_constructor_fields> (@name "indirect-wasi:http/types-[constructor]fields") (;1;) (type 1) (param i32 i32) (result i32)
      local.get 0
      local.get 1
      i32.const 1
      call_indirect (type 1)
    )
    (func $#func2<indirect-wasi:http/types-_constructor_outgoing-request> (@name "indirect-wasi:http/types-[constructor]outgoing-request") (;2;) (type 2) (param i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32) (result i32)
      local.get 0
      local.get 1
      local.get 2
      local.get 3
      local.get 4
      local.get 5
      local.get 6
      local.get 7
      local.get 8
      local.get 9
      local.get 10
      local.get 11
      local.get 12
      local.get 13
      i32.const 2
      call_indirect (type 2)
    )
    (func $#func3<indirect-wasi:http/types-_method_outgoing-request.write> (@name "indirect-wasi:http/types-[method]outgoing-request.write") (;3;) (type 3) (param i32 i32)
      local.get 0
      local.get 1
      i32.const 3
      call_indirect (type 3)
    )
    (func $#func4<indirect-wasi:http/types-_method_outgoing-response.write> (@name "indirect-wasi:http/types-[method]outgoing-response.write") (;4;) (type 3) (param i32 i32)
      local.get 0
      local.get 1
      i32.const 4
      call_indirect (type 3)
    )
    (func $#func5<indirect-wasi:http/types-_method_outgoing-body.write> (@name "indirect-wasi:http/types-[method]outgoing-body.write") (;5;) (type 3) (param i32 i32)
      local.get 0
      local.get 1
      i32.const 5
      call_indirect (type 3)
    )
    (table (;0;) 6 6 funcref)
    (export "$outputstream_blockingwriteandflush" (func $#func0<indirect-wasi:io/streams-_method_output-stream.blocking-write-and-flush>))
    (export "$fields_ctor" (func $#func1<indirect-wasi:http/types-_constructor_fields>))
    (export "$outgoingrequest_ctor" (func $#func2<indirect-wasi:http/types-_constructor_outgoing-request>))
    (export "$outgoingrequest_write" (func $#func3<indirect-wasi:http/types-_method_outgoing-request.write>))
    (export "$outgoingresponse_write" (func $#func4<indirect-wasi:http/types-_method_outgoing-response.write>))
    (export "$outgoingbody_write" (func $#func5<indirect-wasi:http/types-_method_outgoing-body.write>))
    (export "$imports" (table 0))
  )
  (core module $shared
    (type (;0;) (func (param i32 i32 i32 i32)))
    (type (;1;) (func (param i32 i32) (result i32)))
    (type (;2;) (func (param i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32) (result i32)))
    (type (;3;) (func (param i32 i32)))
    (import "" "$outputstream_blockingwriteandflush" (func (;0;) (type 0)))
    (import "" "$fields_ctor" (func (;1;) (type 1)))
    (import "" "$outgoingrequest_ctor" (func (;2;) (type 2)))
    (import "" "$outgoingrequest_write" (func (;3;) (type 3)))
    (import "" "$outgoingresponse_write" (func (;4;) (type 3)))
    (import "" "$outgoingbody_write" (func (;5;) (type 3)))
    (import "" "$imports" (table (;0;) 6 6 funcref))
    (elem (;0;) (i32.const 0) func 0 1 2 3 4 5)
  )
  (core instance $indirect (instantiate $indirect-module))
  (core func $stdout_get (canon lower (func $wasi_cli_stdout "get-stdout")))
  (core instance $wasi_cli_stdout
    (export "get-stdout" (func $stdout_get))
  )
  (core func $outputstream_dtor (canon resource.drop $resource_output_stream))
  (core instance $wasi_io_streams
    (export "[resource-drop]output-stream" (func $outputstream_dtor))
    (export "[method]output-stream.blocking-write-and-flush" (func $indirect "$outputstream_blockingwriteandflush"))
  )
  (core func $outgoingrequest_dtor (canon resource.drop $resource_outgoing_request))
  (core func $outgoingresponse_dtor (canon resource.drop $resource_outgoing_response))
  (core func $outgoingbody_dtor (canon resource.drop $resource_outgoing_body))
  (core func $outgoingresponse_ctor (canon lower (func $wasi_http_types "[constructor]outgoing-response")))
  (core instance $wasi_http_types
    (export "[resource-drop]outgoing-request" (func $outgoingrequest_dtor))
    (export "[resource-drop]outgoing-response" (func $outgoingresponse_dtor))
    (export "[resource-drop]outgoing-body" (func $outgoingbody_dtor))
    (export "[constructor]fields" (func $indirect "$fields_ctor"))
    (export "[constructor]outgoing-request" (func $indirect "$outgoingrequest_ctor"))
    (export "[method]outgoing-request.write" (func $indirect "$outgoingrequest_write"))
    (export "[constructor]outgoing-response" (func $outgoingresponse_ctor))
    (export "[method]outgoing-response.write" (func $indirect "$outgoingresponse_write"))
    (export "[method]outgoing-body.write" (func $indirect "$outgoingbody_write"))
  )
  (core instance $i (instantiate $m
      (with "wasi:cli/stdout" (instance $wasi_cli_stdout))
      (with "wasi:io/streams" (instance $wasi_io_streams))
      (with "wasi:http/types" (instance $wasi_http_types))
    )
  )
  (alias core export $i "memory" (core memory $mem))
  (alias core export $i "cabi_realloc" (core func $realloc))
  (alias core export $indirect "$imports" (core table $table))
  (core func $outputstream_blockingwriteandflush (canon lower
    (func $wasi_io_streams "[method]output-stream.blocking-write-and-flush") (memory $mem)
  ))
  (core func $fields_ctor (canon lower
    (func $wasi_http_types "[constructor]fields") (memory $mem) string-encoding=utf8
  ))
  (core func $outgoingrequest_ctor (canon lower
    (func $wasi_http_types "[constructor]outgoing-request") (memory $mem) string-encoding=utf8
  ))
  (core func $outgoingrequest_write (canon lower
    (func $wasi_http_types "[method]outgoing-request.write") (memory $mem)
  ))
  (core func $outgoingresponse_write (canon lower
    (func $wasi_http_types "[method]outgoing-response.write") (memory $mem)
  ))
  (core func $outgoingbody_write (canon lower
    (func $wasi_http_types "[method]outgoing-body.write") (memory $mem)
  ))
  (core instance (instantiate $shared
      (with "" (instance
        (export "$imports" (table $table))
        (export "$outputstream_blockingwriteandflush" (func $outputstream_blockingwriteandflush))
        (export "$fields_ctor" (func $fields_ctor))
        (export "$outgoingrequest_ctor" (func $outgoingrequest_ctor))
        (export "$outgoingrequest_write" (func $outgoingrequest_write))
        (export "$outgoingresponse_write" (func $outgoingresponse_write))
        (export "$outgoingbody_write" (func $outgoingbody_write))
      ))
    )
  )

  (func $run (result (result))
    (canon lift (core func $i "run")))
  (instance (export (interface "wasi:cli/run"))
    (export "run" (func $run)))
)
