;;! gc = true

;; Structs with GC reference fields survive GC correctly.

(module
  (type $inner (struct (field i32)))
  (type $outer (struct (field (ref null $inner)) (field i32)))

  (import "wasmtime" "gc" (func $gc))

  (func (export "test") (result i32)
    (local $a (ref null $inner))
    (local $b (ref null $outer))

    ;; Create inner struct with value 42
    (local.set $a (struct.new $inner (i32.const 42)))

    ;; Force GC - $a should be relocated
    (call $gc)

    ;; Create outer struct referencing inner, with value 100
    (local.set $b (struct.new $outer (local.get $a) (i32.const 100)))

    ;; Force GC - both $a and $b should be relocated
    (call $gc)

    ;; Read inner.field through outer.field_0
    (i32.add
      (struct.get $inner 0 (struct.get $outer 0 (local.get $b)))
      (struct.get $outer 1 (local.get $b))
    )
  )
)

(assert_return (invoke "test") (i32.const 142))
