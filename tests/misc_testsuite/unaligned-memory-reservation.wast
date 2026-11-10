;;! custom_page_sizes = true
;;! multi_memory = true
;;! flags = "-O memory-reservation=131071 -O memory-may-move=n -O pooling-max-memory-size=131071"

;; A memory that may not move can't grow beyond its reservation, even when the
;; reservation isn't a multiple of the host page size. Every byte up to the
;; memory's current size is accessible, and every byte beyond it traps.

(module
  (memory $memory 1)
  (memory $memory1 65536 (pagesize 1))
  (func (export "grow") (param i32) (result i32)
    (memory.grow $memory (local.get 0)))
  (func (export "grow1") (param i32) (result i32)
    (memory.grow $memory1 (local.get 0)))
  (func (export "size1") (result i32)
    (memory.size $memory1))
  (func (export "load") (param i32) (result i32)
    (i32.load $memory (local.get 0)))
  (func (export "load8") (param i32) (result i32)
    (i32.load8_u $memory (local.get 0)))
  (func (export "load8_1") (param i32) (result i32)
    (i32.load8_u $memory1 (local.get 0)))
  (func (export "load_1") (param i32) (result i32)
    (i32.load $memory1 (local.get 0)))
)

;; Two 64KiB pages exceed the 131071-byte reservation.
(assert_return (invoke "grow" (i32.const 1)) (i32.const -1))
(assert_return (invoke "load" (i32.const 65532)) (i32.const 0))
(assert_return (invoke "load8" (i32.const 65535)) (i32.const 0))
(assert_trap (invoke "load" (i32.const 65533)) "out of bounds memory access")
(assert_trap (invoke "load8" (i32.const 65536)) "out of bounds memory access")

;; A 1-byte-page memory grows to exactly the reservation, and no further.
(assert_return (invoke "grow1" (i32.const 65535)) (i32.const 65536))
(assert_return (invoke "size1") (i32.const 131071))
(assert_return (invoke "grow1" (i32.const 1)) (i32.const -1))
(assert_return (invoke "load_1" (i32.const 131067)) (i32.const 0))
(assert_return (invoke "load8_1" (i32.const 131070)) (i32.const 0))
(assert_trap (invoke "load_1" (i32.const 131068)) "out of bounds memory access")
(assert_trap (invoke "load8_1" (i32.const 131071)) "out of bounds memory access")
