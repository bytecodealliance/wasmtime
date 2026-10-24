;;! threads = true

;; A shared memory defined in one instance and imported into another must
;; still report an out-of-bounds access as a trap.
(module $a
  (memory (export "m") 1 1 shared)
  (func (export "ld") (param i32) (result i32) (i32.load (local.get 0))))
(register "a" $a)
(module $b (import "a" "m" (memory 1 1 shared)))
(assert_trap (invoke $a "ld" (i32.const 0x10000)) "out of bounds memory access")
