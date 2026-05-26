;;! gc = true

;; Tests that the `ref.test` implementation's loads the VMGcKind and
;; VMSharedTypeIndex from a GC object's header do not reuse stale loads via GVN
;; misoptimization, which could lead to the type check producing a wrong result.

(module $a
  (type $s (struct (field i32)))
  (type $arr (array i32))
  (global (export "gs") (ref $s) (struct.new $s (i32.const 1)))
  (global (export "ga") (ref $arr) (array.new $arr (i32.const 2) (i32.const 3)))
)
(register "a" $a)

;; Abstract type.
(module
  (type $s (struct (field i32)))

  (import "wasmtime" "gc" (func $gc))
  (import "a" "gs" (global $gs (ref $s)))

  (func $assert_eq (param i32 i32)
    (if (i32.eq (local.get 0) (local.get 1))
      (then (return)))
    unreachable
  )

  (func (export "run_kind")
    (loop $loop
      ;; Before GC: the struct should pass ref.test for structref.
      (call $assert_eq
        (ref.test (ref struct) (global.get $gs))
        (i32.const 1))

      (call $gc)

      ;; Make sure that the CFG has a loop, but don't actually take it.
      (if (i32.const 0)
        (then (br $loop)))

      ;; After GC: must still pass.
      (call $assert_eq
        (ref.test (ref struct) (global.get $gs))
        (i32.const 1))
      return
    )
  )
)

(assert_return (invoke "run_kind"))

;; Concrete type.
(module
  (type $s (struct (field i32)))
  (type $s2 (struct (field i32) (field i32)))

  (import "wasmtime" "gc" (func $gc))
  (import "a" "gs" (global $gs (ref $s)))

  (func $assert_eq (param i32 i32)
    (if (i32.eq (local.get 0) (local.get 1))
      (then (return)))
    unreachable
  )

  (func (export "run_concrete")
    (loop $loop
      ;; Before GC: should be a $s.
      (call $assert_eq
        (ref.test (ref $s) (global.get $gs))
        (i32.const 1))

      ;; Also NOT a $s2 (different concrete type).
      (call $assert_eq
        (ref.test (ref $s2) (global.get $gs))
        (i32.const 0))

      (call $gc)

      ;; Make sure that the CFG has a loop, but don't actually take it.
      (if (i32.const 0)
        (then (br $loop)))

      ;; After GC: same results.
      (call $assert_eq
        (ref.test (ref $s) (global.get $gs))
        (i32.const 1))
      (call $assert_eq
        (ref.test (ref $s2) (global.get $gs))
        (i32.const 0))
      return
    )
  )
)

(assert_return (invoke "run_concrete"))
