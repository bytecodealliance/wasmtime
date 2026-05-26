;;! gc = true

(module
  (type $box (struct (field i32)))
  (type $arr (array (ref null $box)))

  (import "wasmtime" "gc" (func $gc))

  (global $g (mut (ref null $arr)) (ref.null $arr))

  (func (export "test") (result i32)
    (local $a (ref null $arr))
    (local $i i32)
    (local $total i32)

    ;; Create array of 10 refs
    (local.set $a
      (array.new_fixed $arr 5
        (struct.new $box (i32.const 1))
        (struct.new $box (i32.const 2))
        (struct.new $box (i32.const 3))
        (struct.new $box (i32.const 4))
        (struct.new $box (i32.const 5))
      )
    )

    ;; GC and allocate to cause heap-growth pressure.
    (call $gc)
    (global.set $g
      (array.new_default $arr (i32.const 10000))
    )
    (call $gc)

    ;; Read back from the array
    (local.set $i (i32.const 0))
    (block $done2
      (loop $loop2
        (br_if $done2 (i32.ge_u (local.get $i) (i32.const 5)))
        (local.set $total
          (i32.add
            (local.get $total)
            (struct.get $box 0 (array.get $arr (local.get $a) (local.get $i)))
          )
        )
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $loop2)
      )
    )
    (local.get $total)
  )
)

;; 1+2+3+4+5=15
(assert_return (invoke "test") (i32.const 15))
