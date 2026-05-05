;;! gc = true

;; Test `array.len` on various array sizes.

(module
  (type $arr-i32 (array (mut i32)))
  (type $arr-ref (array (mut anyref)))

  (func (export "len-zero") (result i32)
    (array.len (array.new_default $arr-i32 (i32.const 0)))
  )

  (func (export "len-one") (result i32)
    (array.len (array.new_default $arr-i32 (i32.const 1)))
  )

  (func (export "len-many") (result i32)
    (array.len (array.new_default $arr-i32 (i32.const 100)))
  )

  (func (export "len-fixed") (result i32)
    (array.len
      (array.new_fixed $arr-i32 4
        (i32.const 10)
        (i32.const 20)
        (i32.const 30)
        (i32.const 40)
      )
    )
  )

  ;; array.len should not change after set operations
  (func (export "len-after-set") (result i32)
    (local $a (ref $arr-i32))
    (local.set $a (array.new_default $arr-i32 (i32.const 5)))
    (array.set $arr-i32 (local.get $a) (i32.const 0) (i32.const 99))
    (array.set $arr-i32 (local.get $a) (i32.const 4) (i32.const 99))
    (array.len (local.get $a))
  )

  ;; array.len on a ref-element array
  (func (export "len-ref-array") (result i32)
    (array.len (array.new_default $arr-ref (i32.const 7)))
  )
)

(assert_return (invoke "len-zero") (i32.const 0))
(assert_return (invoke "len-one") (i32.const 1))
(assert_return (invoke "len-many") (i32.const 100))
(assert_return (invoke "len-fixed") (i32.const 4))
(assert_return (invoke "len-after-set") (i32.const 5))
(assert_return (invoke "len-ref-array") (i32.const 7))

;; array.len on a null array reference should trap
(module
  (type $arr (array i32))
  (func (export "len-null")
    (drop (array.len (ref.null $arr)))
  )
)
(assert_trap (invoke "len-null") "null array reference")
