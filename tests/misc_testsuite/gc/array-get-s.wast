;;! gc = true

;; Test `array.get_s` sign extension for packed `i8` and `i16` element types.

;; i8 sign extension
(module
  (type $arr (array (mut i8)))

  ;; 0x80 as signed is -128
  (func (export "get-s-neg-i8") (result i32)
    (local $a (ref $arr))
    (local.set $a (array.new_default $arr (i32.const 1)))
    (array.set $arr (local.get $a) (i32.const 0) (i32.const 0x80))
    (array.get_s $arr (local.get $a) (i32.const 0))
  )

  ;; 0xff as signed is -1
  (func (export "get-s-neg1-i8") (result i32)
    (local $a (ref $arr))
    (local.set $a (array.new_default $arr (i32.const 1)))
    (array.set $arr (local.get $a) (i32.const 0) (i32.const 0xff))
    (array.get_s $arr (local.get $a) (i32.const 0))
  )

  ;; 0x7f as signed is 127
  (func (export "get-s-pos-i8") (result i32)
    (local $a (ref $arr))
    (local.set $a (array.new_default $arr (i32.const 1)))
    (array.set $arr (local.get $a) (i32.const 0) (i32.const 0x7f))
    (array.get_s $arr (local.get $a) (i32.const 0))
  )
)

(assert_return (invoke "get-s-neg-i8") (i32.const -128))
(assert_return (invoke "get-s-neg1-i8") (i32.const -1))
(assert_return (invoke "get-s-pos-i8") (i32.const 127))

;; i16 sign extension
(module
  (type $arr (array (mut i16)))

  ;; 0x8000 as signed is -32768
  (func (export "get-s-neg-i16") (result i32)
    (local $a (ref $arr))
    (local.set $a (array.new_default $arr (i32.const 1)))
    (array.set $arr (local.get $a) (i32.const 0) (i32.const 0x8000))
    (array.get_s $arr (local.get $a) (i32.const 0))
  )

  ;; 0xffff as signed is -1
  (func (export "get-s-neg1-i16") (result i32)
    (local $a (ref $arr))
    (local.set $a (array.new_default $arr (i32.const 1)))
    (array.set $arr (local.get $a) (i32.const 0) (i32.const 0xffff))
    (array.get_s $arr (local.get $a) (i32.const 0))
  )

  ;; 0x7fff as signed is +32767
  (func (export "get-s-pos-i16") (result i32)
    (local $a (ref $arr))
    (local.set $a (array.new_default $arr (i32.const 1)))
    (array.set $arr (local.get $a) (i32.const 0) (i32.const 0x7fff))
    (array.get_s $arr (local.get $a) (i32.const 0))
  )
)

(assert_return (invoke "get-s-neg-i16") (i32.const -32768))
(assert_return (invoke "get-s-neg1-i16") (i32.const -1))
(assert_return (invoke "get-s-pos-i16") (i32.const 32767))
