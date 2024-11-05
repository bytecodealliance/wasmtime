;;! memory64 = true

(module
  (memory i64 1)
  (func (export "load1") (result i32)
      i64.const 0xffff_ffff_ffff_fff0
      i32.load offset=16)
  (func (export "load2") (result i32)
      i64.const 16
      i32.load offset=0xfffffffffffffff0)
)
(assert_trap (invoke "load1") "out of bounds memory access")
(assert_trap (invoke "load2") "out of bounds memory access")
