(module
  (func (export "if_b20") (param i32) (result i32)
    (i32.and (local.get 0) (i32.shl (i32.const 1) (i32.const 20)))
    if (result i32)
      i32.const 100
    else
      i32.const 200
    end
  )
  (func (export "select_b20") (param i32 i32 i32) (result i32)
    local.get 1
    local.get 2
    (i32.and (local.get 0) (i32.shl (i32.const 1) (i32.const 20)))
    select
  )
  (func (export "eqz_b20") (param i32) (result i32)
    (i32.and (local.get 0) (i32.shl (i32.const 1) (i32.const 20)))
    i32.eqz
  )

  (func (export "if_b40") (param i64) (result i64)
    (i64.and (local.get 0) (i64.shl (i64.const 1) (i64.const 40)))
    i64.const 0
    i64.ne
    if (result i64)
      i64.const 100
    else
      i64.const 200
    end
  )
  (func (export "select_b40") (param i64 i64 i64) (result i64)
    local.get 1
    local.get 2
    (i64.and (local.get 0) (i64.shl (i64.const 1) (i64.const 40)))
    i64.const 0
    i64.ne
    select
  )
  (func (export "eqz_b40") (param i64) (result i32)
    (i64.and (local.get 0) (i64.shl (i64.const 1) (i64.const 40)))
    i64.eqz
  )

  (func (export "if_bit32") (param i32 i32) (result i32)
    (i32.and (local.get 0) (i32.shl (i32.const 1) (local.get 1)))
    if (result i32)
      i32.const 100
    else
      i32.const 200
    end
  )

  (func (export "if_bit64") (param i64 i64) (result i64)
    (i64.and (local.get 0) (i64.shl (i64.const 1) (local.get 1)))
    i64.const 0
    i64.ne
    if (result i64)
      i64.const 100
    else
      i64.const 200
    end
  )
)

(assert_return (invoke "if_b20" (i32.const 0)) (i32.const 200))
(assert_return (invoke "if_b20" (i32.const 0x100000)) (i32.const 100))
(assert_return (invoke "select_b20" (i32.const 0) (i32.const 100) (i32.const 200)) (i32.const 200))
(assert_return (invoke "select_b20" (i32.const 0x100000) (i32.const 100) (i32.const 200)) (i32.const 100))
(assert_return (invoke "eqz_b20" (i32.const 0)) (i32.const 1))
(assert_return (invoke "eqz_b20" (i32.const 0x100000)) (i32.const 0))

(assert_return (invoke "if_b40" (i64.const 0)) (i64.const 200))
(assert_return (invoke "if_b40" (i64.const 0x10000000000)) (i64.const 100))
(assert_return (invoke "select_b40" (i64.const 0) (i64.const 100) (i64.const 200)) (i64.const 200))
(assert_return (invoke "select_b40" (i64.const 0x10000000000) (i64.const 100) (i64.const 200)) (i64.const 100))
(assert_return (invoke "eqz_b40" (i64.const 0)) (i32.const 1))
(assert_return (invoke "eqz_b40" (i64.const 0x10000000000)) (i32.const 0))

(assert_return (invoke "if_bit32" (i32.const 0) (i32.const 1)) (i32.const 200))
(assert_return (invoke "if_bit32" (i32.const 0) (i32.const 0)) (i32.const 200))
(assert_return (invoke "if_bit32" (i32.const 1) (i32.const 1)) (i32.const 200))
(assert_return (invoke "if_bit32" (i32.const 1) (i32.const 33)) (i32.const 200))
(assert_return (invoke "if_bit32" (i32.const 1) (i32.const 0)) (i32.const 100))
(assert_return (invoke "if_bit32" (i32.const 1) (i32.const 32)) (i32.const 100))
(assert_return (invoke "if_bit32" (i32.const 0x100000) (i32.const 20)) (i32.const 100))
(assert_return (invoke "if_bit32" (i32.const 0x100000) (i32.const 52)) (i32.const 100))

(assert_return (invoke "if_bit64" (i64.const 0) (i64.const 1)) (i64.const 200))
(assert_return (invoke "if_bit64" (i64.const 0) (i64.const 0)) (i64.const 200))
(assert_return (invoke "if_bit64" (i64.const 1) (i64.const 1)) (i64.const 200))
(assert_return (invoke "if_bit64" (i64.const 1) (i64.const 33)) (i64.const 200))
(assert_return (invoke "if_bit64" (i64.const 1) (i64.const 0)) (i64.const 100))
(assert_return (invoke "if_bit64" (i64.const 1) (i64.const 64)) (i64.const 100))
(assert_return (invoke "if_bit64" (i64.const 0x100000) (i64.const 20)) (i64.const 100))
(assert_return (invoke "if_bit64" (i64.const 0x100000) (i64.const 52)) (i64.const 200))
(assert_return (invoke "if_bit64" (i64.const 0x100000) (i64.const 84)) (i64.const 100))
