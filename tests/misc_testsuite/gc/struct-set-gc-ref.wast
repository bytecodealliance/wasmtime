;;! gc = true

(module
  (type $inner (struct (field i32)))
  (type $outer (struct (field (mut (ref null $inner)))))

  (import "wasmtime" "gc" (func $gc))

  (func (export "test") (result i32)
    (local $o (ref null $outer))
    (local $i (ref null $inner))
    ;; Create outer with null inner
    (local.set $o (struct.new $outer (ref.null $inner)))
    (call $gc)
    ;; Create inner
    (local.set $i (struct.new $inner (i32.const 999)))
    (call $gc)
    ;; Set the field
    (struct.set $outer 0 (local.get $o) (local.get $i))
    ;; Trigger GC
    (call $gc)
    ;; Read the field
    (struct.get $inner 0 (struct.get $outer 0 (local.get $o)))
  )
)

(assert_return (invoke "test") (i32.const 999))
