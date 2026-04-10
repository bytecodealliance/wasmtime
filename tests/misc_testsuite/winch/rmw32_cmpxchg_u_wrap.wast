;;! threads = true

(module
  (memory 1 1 shared)
  (func (export "f") (result i64)
    i32.const 0
    i64.const 0xDEADBEEF00000000
    i64.const 0x1234
    i64.atomic.rmw32.cmpxchg_u))

(assert_return (invoke "f") (i64.const 0))
