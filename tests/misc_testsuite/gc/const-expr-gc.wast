;;! gc = true

;; Test GC operations in constant expression contexts (global initializers and
;; element segment expressions) and covers `struct.new_default` with every
;; storage type to cover all our code paths.

;; `ConstOp::StructNew`
(module
  (type $s (struct (field i32) (field f64)))

  (global $g anyref (struct.new $s (i32.const 7) (f64.const 3.14)))

  (func (export "struct-new-const") (result i32)
    (struct.get $s 0 (ref.cast (ref $s) (global.get $g)))
  )
)
(assert_return (invoke "struct-new-const") (i32.const 7))

;; `ConstOp::StructNewDefault` w/ one field per storage type
(module
  (type $s (struct
    (field i8)
    (field i16)
    (field i32)
    (field i64)
    (field f32)
    (field f64)
    (field anyref)
  ))

  ;; struct.new_default fills every field with its zero/null default
  (global $g anyref (struct.new_default $s))

  (func (export "struct-new-default-i32") (result i32)
    (struct.get_s $s 0 (ref.cast (ref $s) (global.get $g)))
  )
  (func (export "struct-new-default-i64") (result i64)
    (struct.get $s 3 (ref.cast (ref $s) (global.get $g)))
  )
  (func (export "struct-new-default-f32") (result f32)
    (struct.get $s 4 (ref.cast (ref $s) (global.get $g)))
  )
  (func (export "struct-new-default-f64") (result f64)
    (struct.get $s 5 (ref.cast (ref $s) (global.get $g)))
  )
  (func (export "struct-new-default-ref-null") (result i32)
    (ref.is_null (struct.get $s 6 (ref.cast (ref $s) (global.get $g))))
  )
)
(assert_return (invoke "struct-new-default-i32") (i32.const 0))
(assert_return (invoke "struct-new-default-i64") (i64.const 0))
(assert_return (invoke "struct-new-default-f32") (f32.const 0.0))
(assert_return (invoke "struct-new-default-f64") (f64.const 0.0))
(assert_return (invoke "struct-new-default-ref-null") (i32.const 1))

;; `ConstOp::ArrayNew`
(module
  (type $arr (array (mut i32)))

  (global $g anyref (array.new $arr (i32.const 99) (i32.const 3)))

  (func (export "array-new-const-len") (result i32)
    (array.len (ref.cast (ref $arr) (global.get $g)))
  )
  (func (export "array-new-const-elem") (result i32)
    (array.get $arr (ref.cast (ref $arr) (global.get $g)) (i32.const 1))
  )
)
(assert_return (invoke "array-new-const-len") (i32.const 3))
(assert_return (invoke "array-new-const-elem") (i32.const 99))

;; `ConstOp::ArrayNewDefault`
(module
  (type $arr (array (mut i32)))

  (global $g anyref (array.new_default $arr (i32.const 4)))

  (func (export "array-new-default-const-len") (result i32)
    (array.len (ref.cast (ref $arr) (global.get $g)))
  )
  (func (export "array-new-default-const-elem") (result i32)
    (array.get $arr (ref.cast (ref $arr) (global.get $g)) (i32.const 2))
  )
)
(assert_return (invoke "array-new-default-const-len") (i32.const 4))
(assert_return (invoke "array-new-default-const-elem") (i32.const 0))

;; `ConstOp::ArrayNewFixed`
(module
  (type $arr (array i32))

  (global $g anyref (array.new_fixed $arr 3
    (i32.const 10)
    (i32.const 20)
    (i32.const 30)
  ))

  (func (export "array-new-fixed-const-len") (result i32)
    (array.len (ref.cast (ref $arr) (global.get $g)))
  )
  (func (export "array-new-fixed-const-elem0") (result i32)
    (array.get $arr (ref.cast (ref $arr) (global.get $g)) (i32.const 0))
  )
  (func (export "array-new-fixed-const-elem2") (result i32)
    (array.get $arr (ref.cast (ref $arr) (global.get $g)) (i32.const 2))
  )
)
(assert_return (invoke "array-new-fixed-const-len")   (i32.const 3))
(assert_return (invoke "array-new-fixed-const-elem0") (i32.const 10))
(assert_return (invoke "array-new-fixed-const-elem2") (i32.const 30))

;; `ConstOp::ExternConvertAny`
(module
  (type $s (struct (field i32)))

  ;; Convert a struct anyref to an externref in a const expression.
  (global $g externref
    (extern.convert_any (struct.new $s (i32.const 55)))
  )

  (func (export "extern-convert-any-const") (result i32)
    (struct.get $s 0
      (ref.cast (ref $s)
        (any.convert_extern (global.get $g))
      )
    )
  )
)
(assert_return (invoke "extern-convert-any-const") (i32.const 55))

;; Null any -> null extern in a const expression.
(module
  (global $g externref (extern.convert_any (ref.null any)))

  (func (export "extern-convert-any-null") (result i32)
    (ref.is_null (global.get $g))
  )
)
(assert_return (invoke "extern-convert-any-null") (i32.const 1))

;; `ConstOp::AnyConvertExtern`
(module
  (global $g anyref
    (any.convert_extern
      (extern.convert_any
        (ref.i31 (i32.const 7))
      )
    )
  )
  (func (export "any-convert-extern-non-null") (result i32)
    (i31.get_u (ref.cast (ref i31) (global.get $g)))
  )
)
(assert_return (invoke "any-convert-extern-non-null") (i32.const 7))

;; Null extern -> null any in a const expression.
(module
  (func (export "any-convert-extern-null") (result i32)
    (ref.is_null (global.get $g))
  )
  (global $g anyref (any.convert_extern (ref.null extern)))
)
(assert_return (invoke "any-convert-extern-null") (i32.const 1))

;; `ConstOp::RefI31`
(module
  (global $g i31ref (ref.i31 (i32.const 42)))

  (func (export "ref-i31-const") (result i32)
    (i31.get_u (global.get $g))
  )
)
(assert_return (invoke "ref-i31-const") (i32.const 42))
