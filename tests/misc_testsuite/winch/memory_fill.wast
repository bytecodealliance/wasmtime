(module
  (memory 1 1)
  
  (func (export "test")
    (memory.fill (i32.const 0xFF00) (i32.const 0x55) (i32.const 256))))
(invoke "test")

(module
  (memory 1 1)
  
  (func (export "test")
    (memory.fill (i32.const 0xFF00) (i32.const 0x55) (i32.const 257))))
(assert_trap (invoke "test") "out of bounds memory access")

(module
  (memory 1 1)
  
  (func (export "test")
    (memory.fill (i32.const 0xFFFFFF00) (i32.const 0x55) (i32.const 257))))
(assert_trap (invoke "test") "out of bounds memory access")

(module
  (memory 1 1)
  
  (func (export "test")
    (memory.fill (i32.const 0x12) (i32.const 0x55) (i32.const 0))))
(invoke "test")

(module
  (memory 1 1)
  
  (func (export "test")
    (memory.fill (i32.const 0x10000) (i32.const 0x55) (i32.const 0))))
(invoke "test")

(module
  (memory 1 1)
  
  (func (export "test")
    (memory.fill (i32.const 0x20000) (i32.const 0x55) (i32.const 0))))
(assert_trap (invoke "test") "out of bounds memory access")

(module
  (memory 1 1)
  
  (func (export "test")
    (memory.fill (i32.const 0x1) (i32.const 0xAA) (i32.const 0xFFFE))))
(invoke "test")


(module
  (memory 1 1)
  
  (func (export "test")
     (memory.fill (i32.const 0x12) (i32.const 0x55) (i32.const 10))
     (memory.fill (i32.const 0x15) (i32.const 0xAA) (i32.const 4))))
(invoke "test")
