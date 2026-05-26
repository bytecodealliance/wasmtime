;;! gc = true

(module
  (type $inner (struct (field i32)))
  (type $mixed (struct
    (field i8)
    (field i16)
    (field (ref null $inner))
    (field i64)
  ))

  (import "wasmtime" "gc" (func $gc))

  (func (export "test") (result i32)
    (local $i (ref null $inner))
    (local $m (ref null $mixed))
    (local.set $i (struct.new $inner (i32.const 99)))
    (call $gc)
    (local.set $m
      (struct.new $mixed
        (i32.const 1)
        (i32.const 2)
        (local.get $i)
        (i64.const 3)
      )
    )
    (call $gc)
    (i32.add
      (i32.add
        (struct.get_u $mixed 0 (local.get $m))
        (struct.get_u $mixed 1 (local.get $m))
      )
      (i32.add
        (struct.get $inner 0 (struct.get $mixed 2 (local.get $m)))
        (i32.wrap_i64 (struct.get $mixed 3 (local.get $m)))
      )
    )
  )
)

;; 1+2+99+3=105
(assert_return (invoke "test") (i32.const 105))
