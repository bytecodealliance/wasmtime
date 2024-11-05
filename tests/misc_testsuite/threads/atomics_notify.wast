;;! threads = true

;; From https://github.com/bytecodealliance/wasmtime/pull/5255
;;

(module
  (memory 1 1)
  (func (export "notify") (result i32) (memory.atomic.notify (i32.const 0) (i32.const -1)))
)

;; notify returns 0 on unshared memories
(assert_return (invoke "notify") (i32.const 0))

(module
  (memory 1 1 shared)
  (func (export "notify_shared") (result i32) (memory.atomic.notify (i32.const 0) (i32.const -1)))
)

;; notify returns 0 with 0 waiters
(assert_return (invoke "notify_shared") (i32.const 0))
