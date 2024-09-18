(module
  (type $ty (struct (field (mut f32))
                    (field (mut i8))
                    (field (mut anyref))))

  (global $g (mut (ref null $ty)) (ref.null $ty))

  ;; Constructors.

  (func $new (param f32 i32 anyref) (result (ref $ty))
    (struct.new $ty (local.get 0) (local.get 1) (local.get 2))
  )
  (func (export "new") (param f32 i32 anyref)
    (global.set $g (call $new (local.get 0) (local.get 1) (local.get 2)))
  )

  (func $new-default (result (ref $ty))
    (struct.new_default $ty)
  )
  (func (export "new-default")
    (global.set $g (call $new-default))
  )

  ;; Getters.

  (func $get-f32 (param (ref null $ty)) (result f32)
    (struct.get $ty 0 (local.get 0))
  )
  (func (export "get-f32") (result f32)
    (call $get-f32 (global.get $g))
  )

  (func $get-s-i8 (param (ref null $ty)) (result i32)
    (struct.get_s $ty 1 (local.get 0))
  )
  (func (export "get-s-i8") (result i32)
    (call $get-s-i8 (global.get $g))
  )

  (func $get-u-i8 (param (ref null $ty)) (result i32)
    (struct.get_u $ty 1 (local.get 0))
  )
  (func (export "get-u-i8") (result i32)
    (call $get-u-i8 (global.get $g))
  )

  (func $get-anyref (param (ref null $ty)) (result anyref)
    (struct.get $ty 2 (local.get 0))
  )
  (func (export "get-anyref") (result anyref)
    (call $get-anyref (global.get $g))
  )

  ;; Setters.

  (func $set-f32 (param (ref null $ty) f32)
    (struct.set $ty 0 (local.get 0) (local.get 1))
  )
  (func (export "set-f32") (param f32)
    (call $set-f32 (global.get $g) (local.get 0))
  )

  (func $set-i8 (param (ref null $ty) i32)
    (struct.set $ty 1 (local.get 0) (local.get 1))
  )
  (func (export "set-i8") (param i32)
    (call $set-i8 (global.get $g) (local.get 0))
  )

  (func $set-anyref (param (ref null $ty) anyref)
    (struct.set $ty 2 (local.get 0) (local.get 1))
  )
  (func (export "set-anyref") (param anyref)
    (call $set-anyref (global.get $g) (local.get 0))
  )

  (func (export "set-anyref-non-null")
    (call $set-anyref (global.get $g) (struct.new_default $ty))
  )
)

(assert_return (invoke "new" (f32.const 1) (i32.const -1) (ref.null any)))
(assert_return (invoke "get-f32") (f32.const 1))
(assert_return (invoke "get-s-i8") (i32.const -1))
(assert_return (invoke "get-u-i8") (i32.const 255))
(assert_return (invoke "get-anyref") (ref.null any))

(assert_return (invoke "new-default"))
(assert_return (invoke "get-f32") (f32.const 0))
(assert_return (invoke "get-s-i8") (i32.const 0))
(assert_return (invoke "get-u-i8") (i32.const 0))
(assert_return (invoke "get-anyref") (ref.null any))

(assert_return (invoke "set-f32" (f32.const 2)))
(assert_return (invoke "get-f32") (f32.const 2))

(assert_return (invoke "set-i8" (i32.const -1)))
(assert_return (invoke "get-s-i8") (i32.const -1))
(assert_return (invoke "get-u-i8") (i32.const 255))

(assert_return (invoke "set-anyref-non-null"))
(assert_return (invoke "get-anyref") (ref.struct))
(assert_return (invoke "set-anyref" (ref.null any)))
(assert_return (invoke "get-anyref") (ref.null any))

;; Null dereference

(module
  (type $t (struct (field (mut i32) (mut i16))))

  (func (export "struct.get-null") (param (ref null $t))
    (drop (struct.get $t 0 (local.get 0)))
  )

  (func (export "struct.get_s-null") (param (ref null $t))
    (drop (struct.get_s $t 1 (local.get 0)))
  )

  (func (export "struct.get_u-null") (param (ref null $t))
    (drop (struct.get_u $t 1 (local.get 0)))
  )

  (func (export "struct.set-null") (param (ref null $t))
    (struct.set $t 0 (local.get 0) (i32.const 0))
  )
)

(assert_trap (invoke "struct.get-null" (ref.null none)) "null structure reference")
(assert_trap (invoke "struct.get_s-null" (ref.null none)) "null structure reference")
(assert_trap (invoke "struct.get_u-null" (ref.null none)) "null structure reference")
(assert_trap (invoke "struct.set-null" (ref.null none)) "null structure reference")
