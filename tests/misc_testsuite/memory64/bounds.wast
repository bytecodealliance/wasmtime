(assert_unlinkable
  (module
    (memory i64 1)
    (data (i64.const 0xffff_ffff_ffff) "x"))
  "out of bounds memory access")

(module
  (memory i64 1)

  (func (export "copy") (param i64 i64 i64)
      local.get 0
      local.get 1
      local.get 2
      memory.copy)

  (func (export "fill") (param i64 i32 i64)
      local.get 0
      local.get 1
      local.get 2
      memory.fill)

  (func (export "init") (param i64 i32 i32)
      local.get 0
      local.get 1
      local.get 2
      memory.init 0)

  (data "1234")
)

(invoke "copy" (i64.const 0) (i64.const 0) (i64.const 100))
(assert_trap
  (invoke "copy" (i64.const 0x1_0000_0000) (i64.const 0) (i64.const 0))
  "out of bounds memory access")
(assert_trap
  (invoke "copy" (i64.const 0) (i64.const 0x1_0000_0000) (i64.const 0))
  "out of bounds memory access")
(assert_trap
  (invoke "copy" (i64.const 0) (i64.const 0) (i64.const 0x1_0000_0000))
  "out of bounds memory access")

(invoke "fill" (i64.const 0) (i32.const 0) (i64.const 100))
(assert_trap
  (invoke "fill" (i64.const 0x1_0000_0000) (i32.const 0) (i64.const 0))
  "out of bounds memory access")
(assert_trap
  (invoke "fill" (i64.const 0) (i32.const 0) (i64.const 0x1_0000_0000))
  "out of bounds memory access")

(invoke "init" (i64.const 0) (i32.const 0) (i32.const 0))
(invoke "init" (i64.const 0) (i32.const 0) (i32.const 4))
(assert_trap
  (invoke "fill" (i64.const 0x1_0000_0000) (i32.const 0) (i64.const 0))
  "out of bounds memory access")
