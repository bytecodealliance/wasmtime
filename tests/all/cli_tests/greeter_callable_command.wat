;; Like greeter_reactor, but exports "_start" instead of "_initialize".
(module
  (import "wasi_snapshot_preview1" "fd_write"
    (func $__wasi_fd_write (param i32 i32 i32 i32) (result i32)))
  (func (export "_start")
    (call $print (i32.const 32) (i32.const 22))
  )
  (func (export "greet")
    (call $print (i32.const 64) (i32.const 21))
  )
  (func $print (param $ptr i32) (param $len i32)
    (i32.store (i32.const 8) (local.get $len))
    (i32.store (i32.const 4) (local.get $ptr))
        (drop (call $__wasi_fd_write
          (i32.const 1)
          (i32.const 4)
          (i32.const 1)
          (i32.const 0)))
  )
  (memory (export "memory") 1)
  (data (i32.const 32) "Hello callable _start\0a")
  (data (i32.const 64) "Hello callable greet\0a")
)
