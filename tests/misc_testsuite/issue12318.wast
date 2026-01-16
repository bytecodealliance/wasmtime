(module
  (func (export "constants") (result i32)
    i32.const 24
    i32.const 32
    i32.shr_u
    i32.const 1
    i32.const 0
    i32.shl
    i32.or)

  (func (export "variables") (param i32 i32) (result i32)
    local.get 0
    i32.const 32
    i32.shr_u
    local.get 1
    i32.const 0
    i32.shl
    i32.or)
)

(assert_return (invoke "constants") (i32.const 25))
(assert_return (invoke "variables" (i32.const 24) (i32.const 1)) (i32.const 25))
