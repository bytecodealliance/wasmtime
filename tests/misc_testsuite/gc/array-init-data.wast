(module
  (type $arr (array (mut i8)))

  (data $d "abcd")

  (func (export "array-init-data") (param $arr_len i32)
                                   (param $dst i32)
                                   (param $src i32)
                                   (param $data_len i32) (result (ref $arr))
    (local $a (ref $arr))
    (local.set $a (array.new_default $arr (local.get $arr_len)))
    (array.init_data $arr $d (local.get $a) (local.get $dst) (local.get $src) (local.get $data_len))
    (local.get $a)
  )
)

;; In bounds.
(assert_return (invoke "array-init-data" (i32.const 4) (i32.const 0) (i32.const 0) (i32.const 0)) (ref.array))
(assert_return (invoke "array-init-data" (i32.const 4) (i32.const 0) (i32.const 0) (i32.const 4)) (ref.array))
(assert_return (invoke "array-init-data" (i32.const 4) (i32.const 0) (i32.const 1) (i32.const 2)) (ref.array))
(assert_return (invoke "array-init-data" (i32.const 4) (i32.const 0) (i32.const 4) (i32.const 0)) (ref.array))

;; Out-of-bounds data segment accesses.
(assert_trap
  (invoke "array-init-data" (i32.const 5) (i32.const 0) (i32.const 0) (i32.const 5))
  "out of bounds memory access")
(assert_trap
  (invoke "array-init-data" (i32.const 4) (i32.const 0) (i32.const 5) (i32.const 0))
  "out of bounds memory access")
(assert_trap
  (invoke "array-init-data" (i32.const 4) (i32.const 0) (i32.const 1) (i32.const 4))
  "out of bounds memory access")
(assert_trap
  (invoke "array-init-data" (i32.const 4) (i32.const 0) (i32.const 4) (i32.const 1))
  "out of bounds memory access")

;; Out-of-bounds array accesses.
(assert_trap
  (invoke "array-init-data" (i32.const 3) (i32.const 0) (i32.const 0) (i32.const 4))
  "out of bounds array access")
(assert_trap
  (invoke "array-init-data" (i32.const 3) (i32.const 1) (i32.const 0) (i32.const 3))
  "out of bounds array access")
(assert_trap
  (invoke "array-init-data" (i32.const 3) (i32.const 3) (i32.const 0) (i32.const 1))
  "out of bounds array access")
(assert_trap
  (invoke "array-init-data" (i32.const 3) (i32.const 4) (i32.const 0) (i32.const 0))
  "out of bounds array access")

(module
  (type $arr (array (mut i8)))

  (data $d "\aa\bb\cc\dd")

  (func (export "array-init-data-contents") (result i32 i32)
    (local (ref $arr))
    (local.set 0 (array.new_default $arr (i32.const 4)))
    (array.init_data $arr $d (local.get 0) (i32.const 0) (i32.const 1) (i32.const 2))
    (array.get_u $arr (local.get 0) (i32.const 0))
    (array.get_u $arr (local.get 0) (i32.const 1))
  )
)

;; Array is initialized with the correct contents.
(assert_return (invoke "array-init-data-contents") (i32.const 0xbb) (i32.const 0xcc))

(module
  (type $arr (array (mut i32)))

  (data $d "\aa\bb\cc\dd")

  (func (export "array-init-data-little-endian") (result i32)
    (local (ref $arr))
    (local.set 0 (array.new_default $arr (i32.const 1)))
    (array.init_data $arr $d (local.get 0) (i32.const 0) (i32.const 0) (i32.const 1))
    (array.get $arr (local.get 0) (i32.const 0))
  )
)

;; Data segments are interpreted as little-endian.
(assert_return (invoke "array-init-data-little-endian") (i32.const 0xddccbbaa))

(module
  (type $arr (array (mut i16)))

  (data $d "\00\11\22")

  (func (export "array-init-data-unaligned") (result i32)
    (local (ref $arr))
    (local.set 0 (array.new_default $arr (i32.const 1)))
    (array.init_data $arr $d (local.get 0) (i32.const 0) (i32.const 1) (i32.const 1))
    (array.get_u $arr (local.get 0) (i32.const 0))
  )
)

;; Data inside the segment doesn't need to be aligned to the element size.
(assert_return (invoke "array-init-data-unaligned") (i32.const 0x2211))
