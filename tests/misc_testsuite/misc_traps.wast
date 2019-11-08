(module
  (memory 1 1)
  (func (export "load_oob")
    i32.const 65536
    i32.load
    drop
  )
)

(assert_trap (invoke "load_oob") "out of bounds memory access")
(assert_trap (invoke "load_oob") "out of bounds memory access")

(module
  (memory 1 1)
  (func (export "store_oob")
    i32.const 65536
    i32.const 65536
    i32.store
  )
)

(assert_trap (invoke "store_oob") "out of bounds memory access")
(assert_trap (invoke "store_oob") "out of bounds memory access")

(module
  (memory 0 0)
  (func (export "load_oob_0")
    i32.const 0
    i32.load
    drop
  )
)

(assert_trap (invoke "load_oob_0") "out of bounds memory access")
(assert_trap (invoke "load_oob_0") "out of bounds memory access")

(module
  (memory 0 0)
  (func (export "store_oob_0")
    i32.const 0
    i32.const 0
    i32.store
  )
)

(assert_trap (invoke "store_oob_0") "out of bounds memory access")
(assert_trap (invoke "store_oob_0") "out of bounds memory access")

(module
  (func (export "divbyzero") (result i32)
    i32.const 1
    i32.const 0
    i32.div_s
  )
)

(assert_trap (invoke "divbyzero") "integer divide by zero")
(assert_trap (invoke "divbyzero") "integer divide by zero")

(module
  (func (export "unreachable")
    (unreachable)
  )
)

(assert_trap (invoke "unreachable") "unreachable")
(assert_trap (invoke "unreachable") "unreachable")
