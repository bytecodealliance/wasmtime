;;! gc = true
;;! bulk_memory = true

(module
  (type $arr (array i8))
  (data $d "hello world")

  (import "wasmtime" "gc" (func $gc))

  (func (export "test") (result i32)
    (array.new_data $arr $d (i32.const 0) (i32.const 5))

    (call $gc)
    (drop (array.new $arr (i32.const 0) (i32.const 5)))

    (array.get_u $arr (i32.const 0))
  )
)

(assert_return (invoke "test") (i32.const 104)) ;; 'h'
