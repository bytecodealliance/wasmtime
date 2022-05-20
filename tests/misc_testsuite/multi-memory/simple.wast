(module
  (memory $m1 1)
  (memory $m2 1)

  (func (export "store1") (param i32 i64)
      local.get 0
      local.get 1
      i64.store $m1)

  (func (export "store2") (param i32 i64)
      local.get 0
      local.get 1
      i64.store $m2)

  (func (export "load1") (param i32) (result i64)
      local.get 0
      i64.load $m1)

  (func (export "load2") (param i32) (result i64)
      local.get 0
      i64.load $m2)
)

(invoke "store1" (i32.const 0) (i64.const 1))
(invoke "store2" (i32.const 0) (i64.const 2))
(assert_return (invoke "load1" (i32.const 0)) (i64.const 1))
(assert_return (invoke "load2" (i32.const 0)) (i64.const 2))

(module $a
  (memory (export "mem") 1)

  (func (export "store") (param i32 i64)
      local.get 0
      local.get 1
      i64.store)

  (func (export "load") (param i32) (result i64)
      local.get 0
      i64.load)
)

(module $b
  (memory (export "mem") 1)

  (func (export "store") (param i32 i64)
      local.get 0
      local.get 1
      i64.store)

  (func (export "load") (param i32) (result i64)
      local.get 0
      i64.load)
)

(invoke $a "store" (i32.const 0) (i64.const 1))
(invoke $b "store" (i32.const 0) (i64.const 2))
(assert_return (invoke $a "load" (i32.const 0)) (i64.const 1))
(assert_return (invoke $b "load" (i32.const 0)) (i64.const 2))

(module $c
  (import "a" "mem" (memory $m1 1))
  (import "b" "mem" (memory $m2 1))

  (func (export "store1") (param i32 i64)
      local.get 0
      local.get 1
      i64.store $m1)

  (func (export "store2") (param i32 i64)
      local.get 0
      local.get 1
      i64.store $m2)

  (func (export "load1") (param i32) (result i64)
      local.get 0
      i64.load $m1)

  (func (export "load2") (param i32) (result i64)
      local.get 0
      i64.load $m2)
)

(invoke "store1" (i32.const 0) (i64.const 1))
(invoke "store2" (i32.const 0) (i64.const 2))
(assert_return (invoke "load1" (i32.const 0)) (i64.const 1))

(assert_return (invoke "load2" (i32.const 0)) (i64.const 2))

(module
  (memory $m1 1)
  (memory $m2 2)

  (func (export "grow1") (param i32) (result i32)
      local.get 0
      memory.grow $m1)

  (func (export "grow2") (param i32) (result i32)
      local.get 0
      memory.grow $m2)

  (func (export "size1") (result i32) memory.size $m1)
  (func (export "size2") (result i32) memory.size $m2)
)

(assert_return (invoke "size1") (i32.const 1))
(assert_return (invoke "size2") (i32.const 2))
(assert_return (invoke "grow1" (i32.const 3)) (i32.const 1))
(assert_return (invoke "grow1" (i32.const 4)) (i32.const 4))
(assert_return (invoke "grow1" (i32.const 1)) (i32.const 8))
(assert_return (invoke "grow2" (i32.const 1)) (i32.const 2))
(assert_return (invoke "grow2" (i32.const 1)) (i32.const 3))

(module
  (memory $m1 1)
  (memory $m2 1)

  (func (export "init1") (result i32)
      i32.const 1
      i32.const 0
      i32.const 4
      memory.init $m1 $d
      i32.const 1
      i32.load)

  (func (export "init2") (result i32)
      i32.const 1
      i32.const 4
      i32.const 4
      memory.init $m2 $d
      i32.const 1
      i32.load $m2)

  (data $d "\01\00\00\00" "\02\00\00\00")
)

(assert_return (invoke "init1") (i32.const 1))
(assert_return (invoke "init2") (i32.const 2))

(module
  (memory $m1 1)
  (memory $m2 1)

  (func (export "fill1") (result i32)
      i32.const 1
      i32.const 0x01
      i32.const 4
      memory.fill $m1
      i32.const 1
      i32.load)

  (func (export "fill2") (result i32)
      i32.const 1
      i32.const 0x02
      i32.const 2
      memory.fill $m2
      i32.const 1
      i32.load $m2)
)

(assert_return (invoke "fill1") (i32.const 0x01010101))
(assert_return (invoke "fill2") (i32.const 0x0202))
