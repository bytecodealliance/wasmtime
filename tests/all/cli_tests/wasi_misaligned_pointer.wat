(module
  (import "wasi_snapshot_preview1" "fd_write" (func $write (param i32 i32 i32 i32) (result i32)))

  (memory (export "memory") 1)

  (func (export "_start")
    (call $write
      (i32.const 1) ;; fd=1
      (i32.const 1) ;; ciovec_base=1 (misaligned)
      (i32.const 1) ;; ciovec_len=1
      (i32.const 0) ;; retptr=0
    )
    drop

  )
)
