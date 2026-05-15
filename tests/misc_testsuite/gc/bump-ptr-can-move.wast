;;! gc = true

;; Test that the bump pointer load from `VMCopyingHeapData` is not GVN'd
;; or hoisted past a GC call.

(module
  (type $s (struct (field i32)))

  (import "wasmtime" "gc" (func $gc))

  (func $assert_eq (param i32 i32)
    (if (i32.eq (local.get 0) (local.get 1))
      (then (return)))
    unreachable
  )

  ;; Allocate objects in a loop, trigger GC after each one, and verify that
  ;; each freshly allocated object has the correct value. If the bump pointer
  ;; load were incorrectly hoisted, successive allocations could collide.
  (func (export "test")
    (local $i i32)
    (local $ref (ref null $s))

    (loop $l
      ;; Each iteration uses a different value to make corruption detectable.
      (local.set $ref
        (struct.new $s (i32.add (local.get $i) (i32.const 100))))

      (call $gc)

      (call $assert_eq
        (struct.get $s 0 (local.get $ref))
        (i32.add (local.get $i) (i32.const 100)))

      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (br_if $l (i32.lt_u (local.get $i) (i32.const 50)))
    )
  )
)

(assert_return (invoke "test"))
