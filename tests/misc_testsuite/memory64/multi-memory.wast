;;! memory64 = true
;;! multi_memory = true

;; 64 => 64
(module
  (memory $a i64 1)
  (memory $b i64 1)

  (func (export "copy") (param i64 i64 i64)
      local.get 0
      local.get 1
      local.get 2
      memory.copy $a $b)
)
(invoke "copy" (i64.const 0) (i64.const 0) (i64.const 100))
(assert_trap
  (invoke "copy" (i64.const 0x1_0000_0000) (i64.const 0) (i64.const 0))
  "out of bounds memory access")

;; 32 => 64
(module
  (memory $a i32 1)
  (memory $b i64 1)

  (func (export "copy") (param i32 i64 i32)
      local.get 0
      local.get 1
      local.get 2
      memory.copy $a $b)
)
(invoke "copy" (i32.const 0) (i64.const 0) (i32.const 100))
(assert_trap
  (invoke "copy" (i32.const 0) (i64.const 0x1_0000_0000) (i32.const 0))
  "out of bounds memory access")

;; 64 => 32
(module
  (memory $a i64 1)
  (memory $b i32 1)

  (func (export "copy") (param i64 i32 i32)
      local.get 0
      local.get 1
      local.get 2
      memory.copy $a $b)
)
(invoke "copy" (i64.const 0) (i32.const 0) (i32.const 100))
(assert_trap
  (invoke "copy" (i64.const 0x1_0000_0000) (i32.const 0) (i32.const 0))
  "out of bounds memory access")

