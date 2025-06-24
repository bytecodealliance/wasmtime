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
)

(assert_return (invoke "if_b20" (i32.const 0)) (i32.const 200))
(assert_return (invoke "if_b20" (i32.const 0x100000)) (i32.const 100))
(assert_return (invoke "select_b20" (i32.const 0) (i32.const 100) (i32.const 200)) (i32.const 200))
(assert_return (invoke "select_b20" (i32.const 0x100000) (i32.const 100) (i32.const 200)) (i32.const 100))
(assert_return (invoke "eqz_b20" (i32.const 0)) (i32.const 1))
(assert_return (invoke "eqz_b20" (i32.const 0x100000)) (i32.const 0))
