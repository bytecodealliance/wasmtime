;;! gc = true
;;! bulk_memory = true

(module
  (type $ft (func (result i32)))
  (type $fbox (struct (field (ref null $ft))))

  (import "wasmtime" "gc" (func $gc))

  (func $f (type $ft) (i32.const 77))
  (elem declare func $f)

  (func (export "test") (result i32)
    (local $b (ref null $fbox))
    (local.set $b (struct.new $fbox (ref.func $f)))
    (call $gc)
    (call_ref $ft (struct.get $fbox 0 (local.get $b)))
  )
)

(assert_return (invoke "test") (i32.const 77))
