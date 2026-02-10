(module
  (func
    i64.const 1
    i64.const 1
    i64.rotl
    i64.const 2
    i64.ne
    if
      unreachable
    end
  )
  (func
    i32.const 1
    i32.const 1
    i32.rotl
    i32.const 2
    i32.ne
    if
      unreachable
    end
  )
  (export "a" (func 0))
  (export "b" (func 1))
)

(assert_return (invoke "a"))
(assert_return (invoke "b"))
