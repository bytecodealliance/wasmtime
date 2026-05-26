;;! gc = true
;;! simd = true

;; Tests for GC const-expression evaluation with SIMD types.

;; Simple v128.const global
(module
  (global $g v128 (v128.const i32x4 1 2 3 4))

  (func (export "v128-simple") (result v128)
    (global.get $g)
  )
)
(assert_return (invoke "v128-simple") (v128.const i32x4 1 2 3 4))

;; struct.new_default with a v128 field
(module
  (type $s (struct (field v128)))

  (global $g anyref (struct.new_default $s))

  (func (export "v128-struct-default") (result v128)
    (struct.get $s 0 (ref.cast (ref $s) (global.get $g)))
  )
)
(assert_return (invoke "v128-struct-default") (v128.const i32x4 0 0 0 0))

;; array.new_fixed with v128.const elements
(module
  (type $arr (array v128))

  (global $g anyref (array.new_fixed $arr 2
    (v128.const i32x4 1 2 3 4)
    (v128.const i32x4 5 6 7 8)
  ))

  (func (export "v128-array-len") (result i32)
    (array.len (ref.cast (ref $arr) (global.get $g)))
  )
  (func (export "v128-array-elem0") (result v128)
    (array.get $arr (ref.cast (ref $arr) (global.get $g)) (i32.const 0))
  )
  (func (export "v128-array-elem1") (result v128)
    (array.get $arr (ref.cast (ref $arr) (global.get $g)) (i32.const 1))
  )
)
(assert_return (invoke "v128-array-len")   (i32.const 2))
(assert_return (invoke "v128-array-elem0") (v128.const i32x4 1 2 3 4))
(assert_return (invoke "v128-array-elem1") (v128.const i32x4 5 6 7 8))
