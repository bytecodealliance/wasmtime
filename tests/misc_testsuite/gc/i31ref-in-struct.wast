;;! gc = true

(module
  (import "wasmtime" "gc" (func $gc))
  (type $box (struct (field (mut anyref))))

  (func (export "test") (result i32)
    (local $b (ref null $box))
    ;; Store an i31ref in the anyref field
    (local.set $b (struct.new $box (ref.i31 (i32.const 42))))
    (call $gc)
    ;; Read it back
    (i31.get_s (ref.cast (ref i31) (struct.get $box 0 (local.get $b))))
  )
)

(assert_return (invoke "test") (i32.const 42))
