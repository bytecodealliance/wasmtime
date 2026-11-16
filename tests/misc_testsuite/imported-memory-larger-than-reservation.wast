;;! memory64 = true
;;! multi_memory = true
;;! flags = "-O memory-reservation=65536 -O memory-may-move=n"

;; An imported memory may be larger than both the importer's declared minimum
;; and the memory reservation. Accesses within its actual size must succeed,
;; even past the importer's minimum and the reservation.
(module $m
  (memory (export "m32") 4)
  (memory (export "m64") i64 4)
  (data (memory 0) (i32.const 262143) "\01")
  (data (memory 1) (i64.const 262143) "\02"))
(register "m" $m)

(module
  (import "m" "m32" (memory 1))
  (import "m" "m64" (memory i64 1))
  (func (export "load32") (param i32) (result i32)
    (i32.load8_u 0 (local.get 0)))
  (func (export "load64") (param i64) (result i32)
    (i32.load8_u 1 offset=1 (local.get 0))))

(assert_return (invoke "load32" (i32.const 0x20000)) (i32.const 0))
(assert_return (invoke "load32" (i32.const 262143)) (i32.const 1))
(assert_return (invoke "load64" (i64.const 0x20000)) (i32.const 0))
(assert_return (invoke "load64" (i64.const 262142)) (i32.const 2))

;; Accesses past the actual memory's size must still trap.
(assert_trap (invoke "load32" (i32.const 262144)) "out of bounds memory access")
(assert_trap (invoke "load64" (i64.const 262143)) "out of bounds memory access")
