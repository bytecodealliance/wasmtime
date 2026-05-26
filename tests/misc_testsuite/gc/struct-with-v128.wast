;;! gc = true
;;! simd = true

(module
  (type $v (struct (field (mut v128)) (field (mut v128))))

  (import "wasmtime" "gc" (func $gc))

  (func (export "test") (result i32)
    (local $s (ref null $v))
    (local.set $s
      (struct.new $v
        (v128.const i32x4 1 2 3 4)
        (v128.const i32x4 10 20 30 40)
      )
    )
    (call $gc)
    ;; Extract and add lanes from both fields
    (i32.add
      (i32x4.extract_lane 0 (struct.get $v 0 (local.get $s)))
      (i32x4.extract_lane 0 (struct.get $v 1 (local.get $s)))
    )
  )
)

(assert_return (invoke "test") (i32.const 11))
