(module
  (table $t 0 funcref)
  (func (export "size") (result i32)
    (table.size $t)
  )
  (func $f (export "grow-by-1") (result i32)
    (table.grow $t (ref.func $f) (i32.const 1))
  )
)

(assert_return (invoke "size") (i32.const 0))
(assert_return (invoke "grow-by-1") (i32.const 0))
(assert_return (invoke "size") (i32.const 1))
