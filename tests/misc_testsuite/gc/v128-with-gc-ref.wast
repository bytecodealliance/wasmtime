;;! gc = true
;;! simd = true

(module
  (type $inner (struct (field i32)))
  (type $outer (struct (field v128) (field (ref null $inner))))

  (import "wasmtime" "gc" (func $gc))

  (func (export "test") (result i32)
    (local $i (ref null $inner))
    (local $o (ref null $outer))
    (local.set $i (struct.new $inner (i32.const 42)))
    (call $gc)
    (local.set $o
      (struct.new $outer
        (v128.const i32x4 100 200 300 400)
        (local.get $i)
      )
    )
    (call $gc)
    (i32.add
      (i32x4.extract_lane 0 (struct.get $outer 0 (local.get $o)))
      (struct.get $inner 0 (struct.get $outer 1 (local.get $o)))
    )
  )
)

(assert_return (invoke "test") (i32.const 142))
