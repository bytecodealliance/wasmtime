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
    (loop $loop
      ;; Initial `global.get` should have the correct value.
      global.get $g
      struct.get $s 0
      i32.const 42
      call $assert_eq

      ;; GC which can relocate the global's object.
      (call $gc)

      ;; Make sure that the safepoint spiller sees the loop as a loop, but don't
      ;; actually take the back edge.
      (if (i32.const 0)
        (then (br $loop)))

      ;; Get the global again and assert it still has the right value. We should
      ;; not incorrectly GVN/LICM the `global.get` across the call to `$gc` when
      ;; we are using a moving collector, which would result in a stale GC
      ;; reference here.
      global.get $g
      struct.get $s 0
      i32.const 42
      call $assert_eq

      return
    )
  )
)

(assert_return (invoke "run"))
