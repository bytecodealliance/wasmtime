;;! gc = true

;; Test for the `active_space_end` field of `VMCopyingHeapData`. It should not
;; be loaded with `readonly` + `can_move` flags. That would be buggy because
;; `active_space_end` changes whenever the semi-spaces flip during GC. With
;; `readonly` + `can_move`, Cranelift's LICM can hoist the load out of a loop,
;; and after a flip the stale value can let the inline allocation fast path
;; bump-allocate past the real end of the active semi-space into the idle
;; semi-space, corrupting data.

(module
  (type $s (struct (field i32)))

  (import "wasmtime" "gc" (func $gc))

  (func $assert_eq (param i32 i32)
    (if (i32.eq (local.get 0) (local.get 1))
      (then (return)))
    unreachable
  )

  (func (export "test")
    (local $i i32)
    (local $ref (ref null $s))

    ;; Flip once so active_space_end = capacity (the larger value).
    (call $gc)

    ;; Allocate + GC in a tight loop. On each iteration we check that the
    ;; struct we just allocated still holds the expected value after a GC.
    (loop $l
      ;; Allocate a struct with sentinel value 42.
      (local.set $ref (struct.new $s (i32.const 42)))

      ;; Force a collection, which may flip the semi-spaces.
      (call $gc)

      ;; The struct should still contain 42.
      (call $assert_eq
        (struct.get $s 0 (local.get $ref))
        (i32.const 42))

      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (br_if $l (i32.lt_u (local.get $i) (i32.const 200)))
    )
  )
)

(assert_return (invoke "test"))
