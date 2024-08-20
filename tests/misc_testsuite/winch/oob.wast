(module
  (memory 1)
  (func (export "foo") (param $i i32)
    i32.const 0
    (local.get $i)
    i32.store8 offset=4294967295
  )
)

(assert_trap (invoke "foo" (i32.const 0)) "out of bounds") 
