;;! gc = true

(module
  (type $packed (struct
    (field i8)
    (field i16)
    (field i32)
    (field i64)
    (field f32)
    (field f64)
  ))

  (import "wasmtime" "gc" (func $gc))

  (func (export "test") (result i32)
    (local $s (ref null $packed))
    (local.set $s
      (struct.new $packed
        (i32.const 1)
        (i32.const 2)
        (i32.const 3)
        (i64.const 4)
        (f32.const 5.0)
        (f64.const 6.0)
      )
    )
    (call $gc)
    ;; Read back all fields and sum the fields
    (i32.add
      (i32.add
        (i32.add
          (struct.get_u $packed 0 (local.get $s))
          (struct.get_u $packed 1 (local.get $s))
        )
        (i32.add
          (struct.get $packed 2 (local.get $s))
          (i32.wrap_i64 (struct.get $packed 3 (local.get $s)))
        )
      )
      (i32.add
        (i32.trunc_f32_u (struct.get $packed 4 (local.get $s)))
        (i32.trunc_f64_u (struct.get $packed 5 (local.get $s)))
      )
    )
  )
)

(assert_return (invoke "test") (i32.const 21))
