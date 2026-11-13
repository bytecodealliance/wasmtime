;;! flags = "-O memory-reservation=0 -O memory-may-move=n"

;; When a memory's minimum size exceeds the memory reservation, the runtime
;; sizes the allocation from the minimum, so compilers must not treat the
;; reservation as an upper bound on accessible memory. Accesses still trap past
;; the minimum.
(module
  (memory 2)
  (func (export "store") (param i32 i32)
    (i32.store (local.get 0) (local.get 1)))
  (func (export "load") (param i32) (result i32)
    (i32.load (local.get 0)))
  (func (export "load-offset") (param i32) (result i32)
    (i32.load offset=70000 (local.get 0))))

(assert_return (invoke "store" (i32.const 0) (i32.const 42)))
(assert_return (invoke "load" (i32.const 0)) (i32.const 42))
(assert_return (invoke "store" (i32.const 70000) (i32.const 7)))
(assert_return (invoke "load-offset" (i32.const 0)) (i32.const 7))
(assert_return (invoke "load" (i32.const 131068)) (i32.const 0))

;; Accesses past the minimum size must still trap.
(assert_trap (invoke "load" (i32.const 131069)) "out of bounds memory access")
(assert_trap (invoke "load-offset" (i32.const 61069))
  "out of bounds memory access")
