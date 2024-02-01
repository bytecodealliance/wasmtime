(module
  (func $foo
    (call $foo)
  )
  (func (export "stack_overflow")
    (call $foo)
  )
)

(assert_exhaustion (invoke "stack_overflow") "call stack exhausted")
(assert_exhaustion (invoke "stack_overflow") "call stack exhausted")

(module
  (func $foo
    (call $bar)
  )
  (func $bar
    (call $foo)
  )
  (func (export "stack_overflow")
    (call $foo)
  )
)

(assert_exhaustion (invoke "stack_overflow") "call stack exhausted")
(assert_exhaustion (invoke "stack_overflow") "call stack exhausted")
