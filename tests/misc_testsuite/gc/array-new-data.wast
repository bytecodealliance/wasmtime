;;! gc = true

(module
  (type $arr (array (mut i8)))

  (data $d "abcd")

  (func (export "array-new-data") (param i32 i32) (result (ref $arr))
    (array.new_data $arr $d (local.get 0) (local.get 1))
  )
)

;; In-bounds data segment accesses.
(assert_return (invoke "array-new-data" (i32.const 0) (i32.const 0)) (ref.array))
(assert_return (invoke "array-new-data" (i32.const 0) (i32.const 4)) (ref.array))
(assert_return (invoke "array-new-data" (i32.const 1) (i32.const 2)) (ref.array))
(assert_return (invoke "array-new-data" (i32.const 4) (i32.const 0)) (ref.array))

;; Out-of-bounds data segment accesses.
(assert_trap (invoke "array-new-data" (i32.const 0) (i32.const 5)) "out of bounds memory access")
(assert_trap (invoke "array-new-data" (i32.const 5) (i32.const 0)) "out of bounds memory access")
(assert_trap (invoke "array-new-data" (i32.const 1) (i32.const 4)) "out of bounds memory access")
(assert_trap (invoke "array-new-data" (i32.const 4) (i32.const 1)) "out of bounds memory access")


(module
  (type $arr (array (mut i8)))

  (data $d "\aa\bb\cc\dd")

  (func (export "array-new-data-contents") (result i32 i32)
    (local (ref $arr))
    (local.set 0 (array.new_data $arr $d (i32.const 1) (i32.const 2)))
    (array.get_u $arr (local.get 0) (i32.const 0))
    (array.get_u $arr (local.get 0) (i32.const 1))
  )
)

;; Array is initialized with the correct contents.
(assert_return (invoke "array-new-data-contents") (i32.const 0xbb) (i32.const 0xcc))

(module
  (type $arr (array (mut i32)))

  (data $d "\aa\bb\cc\dd")

  (func (export "array-new-data-little-endian") (result i32)
    (array.get $arr
               (array.new_data $arr $d (i32.const 0) (i32.const 1))
               (i32.const 0))
  )
)

;; Data segments are interpreted as little-endian.
(assert_return (invoke "array-new-data-little-endian") (i32.const 0xddccbbaa))

(module
  (type $arr (array (mut i16)))

  (data $d "\00\11\22")

  (func (export "array-new-data-unaligned") (result i32)
    (array.get_u $arr
                 (array.new_data $arr $d (i32.const 1) (i32.const 1))
                 (i32.const 0))
  )
)

;; Data inside the segment doesn't need to be aligned to the element size.
(assert_return (invoke "array-new-data-unaligned") (i32.const 0x2211))
