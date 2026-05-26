(module
  (memory 1)
  (data (i32.const 0) "\ff")
  (data (i32.const 99) "\de\ad\be\ef")
  (func (export "exploit") (result i32)
    i32.const 0
    i32.load8_s
    i32.load offset=100
  )
)
(assert_trap (invoke "exploit") "out of bounds memory access")
