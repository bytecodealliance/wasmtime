;;! threads = true

(module
  (memory (export "mem") 1 1 shared)
  (func (export "notify_last") (result i32)
    (memory.atomic.notify (i32.const 65532) (i32.const 0))
  )
  (func (export "wait_last32") (result i32)
    (memory.atomic.wait32 (i32.const 65532) (i32.const 0) (i64.const 0))
  )
  (func (export "wait_last64") (result i32)
    (memory.atomic.wait64 (i32.const 65528) (i64.const 0) (i64.const 0))
  )
)

(assert_return (invoke "notify_last") (i32.const 0))
(assert_return (invoke "wait_last32") (i32.const 2))
(assert_return (invoke "wait_last64") (i32.const 2))
