(module
  (memory (export "memory0") 2 3)
  (memory (export "memory1") 2 4)

  (func (export "size0") (result i32) (memory.size 0))
  (func (export "load0") (param i32) (result i32)
    local.get 0
    i32.load8_s 0
  )
  (func (export "store0") (param i32 i32)
    local.get 0
    local.get 1
    i32.store8 0
  )
  (func (export "size1") (result i32) (memory.size 1))
  (func (export "load1") (param i32) (result i32)
    local.get 0
    i32.load8_s 1
  )
  (func (export "store1") (param i32 i32)
    local.get 0
    local.get 1
    i32.store8 1
  )

  (data (memory 0) (i32.const 0x1000) "\01\02\03\04")
  (data (memory 1) (i32.const 0x1000) "\04\03\02\01")
)
