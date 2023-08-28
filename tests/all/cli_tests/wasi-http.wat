(module
  (import "wasi_snapshot_preview1" "fd_write"
    (func $__wasi_fd_write (param i32 i32 i32 i32) (result i32)))
  (import "wasi_snapshot_preview1" "proc_exit"
    (func $__wasi_proc_exit (param i32)))
  (import "wasi:http/types" "new-fields"
    (func $__wasi_http_types_new_fields (param i32 i32) (result i32)))
  (import "wasi:http/types" "drop-fields"
    (func $__wasi_http_types_drop_fields (param i32)))

  (func $_start
    (local $i i32)

    (i32.store (i32.const 24) (i32.const 14))
    (i32.store (i32.const 20) (i32.const 0))

    ;; Print "Called _start".
    (call $print (i32.const 32) (i32.const 14))

    (call $__wasi_http_types_new_fields
      (i32.const 0)
      (i32.const 0))
    (call $__wasi_http_types_drop_fields)

    ;; Print "Done".
    (call $print (i32.const 64) (i32.const 5))
  )

  ;; A helper function for printing ptr-len strings.
  (func $print (param $ptr i32) (param $len i32)
    (i32.store (i32.const 8) (local.get $len))
    (i32.store (i32.const 4) (local.get $ptr))
        (drop (call $__wasi_fd_write
          (i32.const 1)
          (i32.const 4)
          (i32.const 1)
          (i32.const 0)))
  )

  (memory 1)
  (export "memory" (memory 0))
  (export "_start" (func $_start))

  (data (i32.const 32) "Called _start\0a")
  (data (i32.const 64) "Done\0a")
)
