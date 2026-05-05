;;! gc = true
;;! bulk_memory = true

;; Test ref.as_non_null: asserts non-null, traps on null.

;; struct
(module
  (type $s (struct (field i32)))

  ;; Returns field value when given a non-null struct.
  (func (export "non-null-struct") (result i32)
    (struct.get $s 0 (ref.as_non_null (struct.new $s (i32.const 42))))
  )

  ;; Traps when given null.
  (func (export "null-struct") (result i32)
    (struct.get $s 0 (ref.as_non_null (ref.null $s)))
  )
)

(assert_return (invoke "non-null-struct") (i32.const 42))
(assert_trap (invoke "null-struct") "null reference")

;; array
(module
  (type $arr (array i32))

  (func (export "non-null-array") (result i32)
    (array.len (ref.as_non_null (array.new_default $arr (i32.const 5))))
  )

  (func (export "null-array") (result i32)
    (array.len (ref.as_non_null (ref.null $arr)))
  )
)

(assert_return (invoke "non-null-array") (i32.const 5))
(assert_trap   (invoke "null-array") "null reference")

;; anyref
(module
  (func (export "non-null-any") (result i32)
    (ref.as_non_null (ref.i31 (i32.const 0)))
    drop
    (i32.const 1)
  )

  (func (export "null-any") (result i32)
    (ref.as_non_null (ref.null any))
    drop
    (i32.const 1)
  )
)

(assert_return (invoke "non-null-any") (i32.const 1))
(assert_trap (invoke "null-any") "null reference")

;; funcref
(module
  (type $ft (func (result i32)))
  (elem func $f)
  (func $f (result i32) (i32.const 7))

  (func (export "non-null-func") (result i32)
    (call_ref $ft (ref.as_non_null (ref.func $f)))
  )

  (func (export "null-func") (result i32)
    (call_ref $ft (ref.as_non_null (ref.null $ft)))
  )
)

(assert_return (invoke "non-null-func") (i32.const 7))
(assert_trap (invoke "null-func") "null reference")
