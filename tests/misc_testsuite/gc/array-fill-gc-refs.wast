;;! gc = true

(module
  (import "wasmtime" "gc" (func $gc))
  (type $box (struct (field i32)))
  (type $arr (array (mut (ref null $box))))

  (func (export "test") (result i32)
    (local $a (ref null $arr))
    (local $b (ref null $box))
    ;; Create box with value 77
    (local.set $b (struct.new $box (i32.const 77)))
    ;; Create array of 5 nulls
    (local.set $a (array.new $arr (ref.null $box) (i32.const 5)))
    ;; Fill all elements with the box
    (array.fill $arr (local.get $a) (i32.const 0) (local.get $b) (i32.const 5))
    ;; Trigger GC
    (call $gc)
    ;; Read element 3
    (struct.get $box 0 (array.get $arr (local.get $a) (i32.const 3)))
  )
)

(assert_return (invoke "test") (i32.const 77))
