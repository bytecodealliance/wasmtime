(module
  (import "wasi_snapshot_preview1" "proc_exit"
    (func $__wasi_proc_exit (param i32)))
  (import "wasi_snapshot_preview1" "fd_write"
    (func $__wasi_fd_write (param i32 i32 i32 i32) (result i32)))
  (func $_start
    (i32.store (i32.const 24) (i32.const 14))
    (i32.store (i32.const 20) (i32.const 0))
    (block
      (br_if 0
        (call $__wasi_fd_write
          (i32.const 1)
          (i32.const 20)
          (i32.const 1)
          (i32.const 16)))
      (br_if 0 (i32.ne (i32.load (i32.const 16)) (i32.const 14)))
      (br 1)
    )
    (call $__wasi_proc_exit (i32.const 1))
  )
  (memory 1)
  (export "memory" (memory 0))
  (export "_start" (func $_start))
  (data (i32.const 0) "Hello, world!\0a")
)
