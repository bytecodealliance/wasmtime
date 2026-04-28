;;! gc = true

(module
  (import "wasmtime" "gc" (func $gc))
  (type $empty (struct))
  (type $holder (struct (field (ref null $empty)) (field i32)))

  (func (export "test") (result i32)
    (local $e (ref null $empty))
    (local $h (ref null $holder))
    (local.set $e (struct.new_default $empty))
    (local.set $h (struct.new $holder (local.get $e) (i32.const 42)))
    (call $gc)
    ;; Check that the empty struct ref is still valid
    (if (ref.is_null (struct.get $holder 0 (local.get $h)))
      (then (return (i32.const -1)))
    )
    (struct.get $holder 1 (local.get $h))
  )
)

(assert_return (invoke "test") (i32.const 42))
