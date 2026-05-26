;;! gc = true

;; Test `br_on_null` and `br_on_non_null` with GC reference types.
;;
;; Note: `assert_return` arguments must be `[type].const` expressions, so GC
;; refs (`struct.new`, `array.new`, `ref.i31`) are created inside the test
;; functions.

;; br_on_null: branch when null, fall through when non-null
(module
  (type $s (struct (field i32)))
  (type $arr (array i32))

  ;; Null struct ref -> branch taken -> returns 0.
  (func (export "br-on-null-struct-null") (result i32)
    (local $r (ref null $s))
    (block $l
      (br_on_null $l (local.get $r))
      (return (i32.const -1))
    )
    (i32.const 0)
  )

  ;; Non-null struct ref -> falls through -> returns -1.
  (func (export "br-on-null-struct-non-null") (result i32)
    (local $r (ref null $s))
    (local.set $r (struct.new_default $s))
    (block $l
      (br_on_null $l (local.get $r))
      (return (i32.const -1))
    )
    (i32.const 0)
  )

  ;; Null array ref -> branch taken -> returns 0.
  (func (export "br-on-null-array-null") (result i32)
    (local $r (ref null $arr))
    (block $l
      (br_on_null $l (local.get $r))
      (return (i32.const -1))
    )
    (i32.const 0)
  )

  ;; Non-null array ref -> falls through -> returns -1.
  (func (export "br-on-null-array-non-null") (result i32)
    (local $r (ref null $arr))
    (local.set $r (array.new_default $arr (i32.const 2)))
    (block $l
      (br_on_null $l (local.get $r))
      (return (i32.const -1))
    )
    (i32.const 0)
  )

  ;; Null i31ref -> branch taken -> returns 0.
  (func (export "br-on-null-i31-null") (result i32)
    (block $l
      (br_on_null $l (ref.null i31))
      (return (i32.const -1))
    )
    (i32.const 0)
  )

  ;; Non-null i31ref -> falls through -> returns -1.
  (func (export "br-on-null-i31-non-null") (result i32)
    (block $l
      (br_on_null $l (ref.i31 (i32.const 5)))
      (return (i32.const -1))
    )
    (i32.const 0)
  )

  ;; When non-null, br_on_null leaves the non-null ref on the stack.
  (func (export "br-on-null-yields-value") (result i32)
    (local $r (ref null $s))
    (local.set $r (struct.new $s (i32.const 42)))
    (block $l
      ;; br_on_null falls through, leaving (ref $s) on stack
      (struct.get $s 0 (br_on_null $l (local.get $r)))
      (return)
    )
    (i32.const -1)
  )
)

(assert_return (invoke "br-on-null-struct-null") (i32.const 0))
(assert_return (invoke "br-on-null-struct-non-null") (i32.const -1))
(assert_return (invoke "br-on-null-array-null") (i32.const 0))
(assert_return (invoke "br-on-null-array-non-null") (i32.const -1))
(assert_return (invoke "br-on-null-i31-null") (i32.const 0))
(assert_return (invoke "br-on-null-i31-non-null") (i32.const -1))
(assert_return (invoke "br-on-null-yields-value") (i32.const 42))

;; br_on_non_null: branch when non-null, fall through when null
(module
  (type $s (struct (field i32)))

  ;; Null struct -> falls through -> returns -1.
  (func (export "br-on-non-null-struct-null") (result i32)
    (local $r (ref null $s))
    (block $l (result (ref $s))
      (br_on_non_null $l (local.get $r))
      (return (i32.const -1))
    )
    (struct.get $s 0)
  )

  ;; Non-null struct -> branch taken -> returns field value.
  (func (export "br-on-non-null-struct-non-null") (result i32)
    (block $l (result (ref $s))
      (br_on_non_null $l (struct.new $s (i32.const 99)))
      (return (i32.const -1))
    )
    (struct.get $s 0)
  )

  ;; Null anyref -> falls through -> returns -1.
  (func (export "br-on-non-null-any-null") (result i32)
    (block $l (result (ref any))
      (br_on_non_null $l (ref.null any))
      (return (i32.const -1))
    )
    (drop)
    (i32.const 1)
  )

  ;; Non-null anyref (i31) -> branch taken -> returns 1.
  (func (export "br-on-non-null-any-non-null") (result i32)
    (block $l (result (ref any))
      (br_on_non_null $l (ref.i31 (i32.const 0)))
      (return (i32.const -1))
    )
    (drop)
    (i32.const 1)
  )
)

(assert_return (invoke "br-on-non-null-struct-null") (i32.const -1))
(assert_return (invoke "br-on-non-null-struct-non-null") (i32.const 99))
(assert_return (invoke "br-on-non-null-any-null") (i32.const -1))
(assert_return (invoke "br-on-non-null-any-non-null") (i32.const 1))
