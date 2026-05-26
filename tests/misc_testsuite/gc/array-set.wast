;;! gc = true

;; Test `array.set` with various element types.

;; i32 elements
(module
  (type $arr (array (mut i32)))

  (func (export "set-and-get") (result i32)
    (local $a (ref $arr))
    (local.set $a (array.new_default $arr (i32.const 3)))
    (array.set $arr (local.get $a) (i32.const 1) (i32.const 42))
    (array.get $arr (local.get $a) (i32.const 1))
  )

  ;; Overwrite an element multiple times; last write wins.
  (func (export "overwrite") (result i32)
    (local $a (ref $arr))
    (local.set $a (array.new_default $arr (i32.const 1)))
    (array.set $arr (local.get $a) (i32.const 0) (i32.const 1))
    (array.set $arr (local.get $a) (i32.const 0) (i32.const 2))
    (array.set $arr (local.get $a) (i32.const 0) (i32.const 3))
    (array.get $arr (local.get $a) (i32.const 0))
  )

  ;; Neighbours of a set element are not disturbed.
  (func (export "neighbours-untouched") (result i32)
    (local $a (ref $arr))
    (local.set $a (array.new $arr (i32.const 0) (i32.const 5)))
    (array.set $arr (local.get $a) (i32.const 2) (i32.const 99))
    ;; Neighbours should still be 0.
    (i32.add
      (array.get $arr (local.get $a) (i32.const 1))
      (array.get $arr (local.get $a) (i32.const 3))
    )
  )
)
(assert_return (invoke "set-and-get") (i32.const 42))
(assert_return (invoke "overwrite") (i32.const 3))
(assert_return (invoke "neighbours-untouched") (i32.const 0))

;; i64 elements
(module
  (type $arr (array (mut i64)))
  (func (export "set-i64") (result i64)
    (local $a (ref $arr))
    (local.set $a (array.new_default $arr (i32.const 2)))
    (array.set $arr (local.get $a) (i32.const 0) (i64.const 0x12345678_87654321))
    (array.get $arr (local.get $a) (i32.const 0))
  )
)
(assert_return (invoke "set-i64") (i64.const 0x12345678_87654321))

;; f32 elements
(module
  (type $arr (array (mut f32)))
  (func (export "set-f32") (result f32)
    (local $a (ref $arr))
    (local.set $a (array.new_default $arr (i32.const 1)))
    (array.set $arr (local.get $a) (i32.const 0) (f32.const 3.14))
    (array.get $arr (local.get $a) (i32.const 0))
  )
)
(assert_return (invoke "set-f32") (f32.const 3.14))

;; f64 elements
(module
  (type $arr (array (mut f64)))
  (func (export "set-f64") (result f64)
    (local $a (ref $arr))
    (local.set $a (array.new_default $arr (i32.const 1)))
    (array.set $arr (local.get $a) (i32.const 0) (f64.const 2.718281828))
    (array.get $arr (local.get $a) (i32.const 0))
  )
)
(assert_return (invoke "set-f64") (f64.const 2.718281828))

;; i8 (packed) elements
(module
  (type $arr (array (mut i8)))
  (func (export "set-i8") (result i32)
    (local $a (ref $arr))
    (local.set $a (array.new_default $arr (i32.const 4)))
    (array.set $arr (local.get $a) (i32.const 0) (i32.const 0xff))
    (array.set $arr (local.get $a) (i32.const 1) (i32.const 0x7f))
    ;; Read back unsigned
    (i32.add
      (array.get_u $arr (local.get $a) (i32.const 0))
      (array.get_u $arr (local.get $a) (i32.const 1))
    )
  )
)
(assert_return (invoke "set-i8") (i32.const 0x17e)) ;; 0xff + 0x7f = 0x17e

;; i16 (packed) elements
(module
  (type $arr (array (mut i16)))
  (func (export "set-i16") (result i32)
    (local $a (ref $arr))
    (local.set $a (array.new_default $arr (i32.const 2)))
    (array.set $arr (local.get $a) (i32.const 0) (i32.const 0x8000))
    (array.set $arr (local.get $a) (i32.const 1) (i32.const 0x7fff))
    (i32.add
      (array.get_u $arr (local.get $a) (i32.const 0))
      (array.get_u $arr (local.get $a) (i32.const 1))
    )
  )
)
(assert_return (invoke "set-i16") (i32.const 0xffff)) ;; 0x8000 + 0x7fff = 0xffff

;; GC-ref elements: write a struct into an anyref array
(module
  (type $box (struct (field i32)))
  (type $arr (array (mut anyref)))

  (func (export "set-gcref") (result i32)
    (local $a (ref $arr))
    (local.set $a (array.new_default $arr (i32.const 3)))
    (array.set $arr (local.get $a) (i32.const 1) (struct.new $box (i32.const 77)))
    (struct.get $box 0
      (ref.cast (ref $box) (array.get $arr (local.get $a) (i32.const 1)))
    )
  )
)
(assert_return (invoke "set-gcref") (i32.const 77))

;; Out-of-bounds set should trap
(module
  (type $arr (array (mut i32)))
  (func (export "oob-set")
    (local $a (ref $arr))
    (local.set $a (array.new_default $arr (i32.const 2)))
    (array.set $arr (local.get $a) (i32.const 2) (i32.const 0))
  )
)
(assert_trap (invoke "oob-set") "out of bounds array access")

;; Set on a null array reference should trap
(module
  (type $arr (array (mut i32)))
  (func (export "null-set")
    (array.set $arr (ref.null $arr) (i32.const 0) (i32.const 0))
  )
)
(assert_trap (invoke "null-set") "null array reference")
