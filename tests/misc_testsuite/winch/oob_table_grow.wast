;;! reference_types = true

(module
  (memory 1)
  (table $t 1 2 funcref)
  (func $exploit (result i32)
    (i32.store offset=1
      (table.grow $t (table.get $t (i32.const 0)) (i32.const 100))
      (i32.const 0xDEADBEEF))
    (i32.load (i32.const 0)))
  (export "exploit" (func $exploit))
)
(assert_trap (invoke "exploit") "out of bounds memory access")
