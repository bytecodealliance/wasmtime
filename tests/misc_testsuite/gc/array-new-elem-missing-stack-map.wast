;;! gc = true
;;! bulk_memory = true

(module
  (type $s (struct (field (mut i32))))
  (type $arr (array (ref null $s)))

  (elem $e (ref null $s) (struct.new $s (i32.const 42)))

  (import "wasmtime" "gc" (func $gc))

  (func (export "test") (result i32)
    (array.new_elem $arr $e (i32.const 0) (i32.const 1))

    (call $gc)
    (drop (struct.new $s (i32.const 0)))
    (drop (struct.new $s (i32.const 0)))
    (drop (struct.new $s (i32.const 0)))
    (drop (struct.new $s (i32.const 0)))
    (drop (struct.new $s (i32.const 0)))

    (struct.get $s 0 (array.get $arr (i32.const 0)))
  )
)

(assert_return (invoke "test") (i32.const 42))
