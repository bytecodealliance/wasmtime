;; Test that we properly canonicalize function types across modules, at the
;; engine level. We rely on this canonicalization to make cross-module imports
;; work among other things.

(module $m1
  ;; A pair of recursive types.
  (rec (type $type_a (sub final (func (result i32 (ref null $type_b)))))
       (type $type_b (sub final (func (result i32 (ref null $type_a))))))

  (func (export "func_a") (type $type_a)
    i32.const 1234
    ref.null $type_b
  )

  (func (export "func_b") (type $type_b)
    i32.const 4321
    ref.null $type_a
  )
)
(register "m1")

(module $m2
  ;; The same pair of recursive types.
  (rec (type $type_a (sub final (func (result i32 (ref null $type_b)))))
       (type $type_b (sub final (func (result i32 (ref null $type_a))))))

  (import "m1" "func_a" (func $func_a (type $type_a)))
  (import "m1" "func_b" (func $func_b (type $type_b)))

  (func (export "call") (result i32 i32)
    call $func_a
    drop
    call $func_b
    drop
  )
)

(assert_return (invoke "call") (i32.const 1234) (i32.const 4321))
