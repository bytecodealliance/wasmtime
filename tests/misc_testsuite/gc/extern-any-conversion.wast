;;! gc = true

;; Test any.convert_extern and extern.convert_any conversions.

;; null roundtrip
(module
  (func (export "null-extern-to-any") (result i32)
    ;; null extern -> any -> check it's null
    (ref.is_null (any.convert_extern (ref.null extern)))
  )

  (func (export "null-any-to-extern") (result i32)
    ;; null any -> extern -> check it's null
    (ref.is_null (extern.convert_any (ref.null any)))
  )
)

(assert_return (invoke "null-extern-to-any") (i32.const 1))
(assert_return (invoke "null-any-to-extern") (i32.const 1))

;; i31ref roundtrip through extern
(module
  (func (export "i31-roundtrip") (result i32)
    ;; ref.i31(42) -> extern -> back to any -> cast to i31 -> get value
    (i31.get_u
      (ref.cast (ref i31)
        (any.convert_extern
          (extern.convert_any
            (ref.i31 (i32.const 42))
          )
        )
      )
    )
  )
)

(assert_return (invoke "i31-roundtrip") (i32.const 42))

;; struct roundtrip through extern
(module
  (type $s (struct (field i32)))

  (func (export "struct-roundtrip") (result i32)
    (struct.get $s 0
      (ref.cast (ref $s)
        (any.convert_extern
          (extern.convert_any
            (struct.new $s (i32.const 77))
          )
        )
      )
    )
  )
)

(assert_return (invoke "struct-roundtrip") (i32.const 77))

;; identity: converting to extern and back gives same GC object
(module
  (type $s (struct))

  (func (export "identity") (result i32)
    (local $orig (ref $s))
    (local $roundtrip (ref null $s))
    (local.set $orig (struct.new_default $s))
    (local.set $roundtrip
      (ref.cast (ref null $s)
        (any.convert_extern
          (extern.convert_any (local.get $orig))
        )
      )
    )
    ;; ref.eq: same object identity
    (ref.eq (local.get $orig) (local.get $roundtrip))
  )
)

(assert_return (invoke "identity") (i32.const 1))

;; extern.convert_any on non-null anyref (i31)
(module
  (func (export "any-to-extern-non-null") (result i32)
    (ref.is_null (extern.convert_any (ref.i31 (i32.const 1))))
  )
)

(assert_return (invoke "any-to-extern-non-null") (i32.const 0))
