;;! gc = true

(module
  (type $box (struct (field i32)))
  (type $arr (array (ref null $box)))

  (import "wasmtime" "gc" (func $gc))

  (func (export "test") (result i32)
    (local $a (ref null $arr))
    (local.set $a
      (array.new_fixed $arr 3
        (block (result (ref $box))
          (struct.new $box (i32.const 10))
          (call $gc)
        )
        (block (result (ref $box))
          (struct.new $box (i32.const 20))
          (call $gc)
        )
        (block (result (ref $box))
          (struct.new $box (i32.const 30))
          (call $gc)
        )
      )
    )
    (call $gc)
    (i32.add
      (i32.add
        (struct.get $box 0 (array.get $arr (local.get $a) (i32.const 0)))
        (struct.get $box 0 (array.get $arr (local.get $a) (i32.const 1)))
      )
      (struct.get $box 0 (array.get $arr (local.get $a) (i32.const 2)))
    )
  )
)

(assert_return (invoke "test") (i32.const 60))
