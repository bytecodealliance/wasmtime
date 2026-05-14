;;! gc = true

(module $a
  (type $s (struct (field i32)))
  (global (export "g") (ref $s) (struct.new $s (i32.const 42)))
)
(register "a" $a)

(module
  (type $s (struct (field i32)))

  (import "wasmtime" "gc" (func $gc))
  (import "a" "g" (global $g (ref $s)))

  (func $assert_eq (param i32 i32)
    (if (i32.eq (local.get 0) (local.get 1))
      (then (return)))
    unreachable
  )

  (func (export "run")
    (local $i i32)

    (loop $outer
      global.get $g
      struct.get $s 0
      i32.const 42
      call $assert_eq

      (call $gc)

      (if (i32.eq (local.get $i) (i32.const -1))
        (then
          (if (i32.lt_u (local.get $i) (i32.const 10))
            (then (br $outer)))
        ))

      global.get $g
      struct.get $s 0
      i32.const 42
      call $assert_eq
      return
    )
  )
)

(assert_return (invoke "run"))
