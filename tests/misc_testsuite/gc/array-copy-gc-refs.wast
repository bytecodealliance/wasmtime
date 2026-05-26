;;! gc = true

(module
  (import "wasmtime" "gc" (func $gc))
  (type $box (struct (field i32)))
  (type $arr (array (mut (ref null $box))))

  (func (export "test") (result i32)
    (local $src (ref null $arr))
    (local $dst (ref null $arr))
    ;; Create source array
    (local.set $src (array.new_fixed $arr 3
      (struct.new $box (i32.const 10))
      (struct.new $box (i32.const 20))
      (struct.new $box (i32.const 30))
    ))
    ;; Create dest array of nulls
    (local.set $dst (array.new $arr (ref.null $box) (i32.const 3)))
    ;; Copy src[0..3] to dst[0..3]
    (array.copy $arr $arr (local.get $dst) (i32.const 0) (local.get $src) (i32.const 0) (i32.const 3))
    ;; Trigger GC
    (call $gc)
    ;; Read from dst
    (i32.add
      (struct.get $box 0 (array.get $arr (local.get $dst) (i32.const 0)))
      (struct.get $box 0 (array.get $arr (local.get $dst) (i32.const 2)))
    )
  )
)

(assert_return (invoke "test") (i32.const 40))
