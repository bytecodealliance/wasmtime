;;! multi_memory = true
;; Test that two imports of the same memory correctly alias.
;; The alias region system should ensure that stores through one import
;; are visible through loads from the other import.

(module $M
  (memory (export "a") (export "b") 1)
)
(register "M")

(module
  (import "M" "a" (memory $a 1))
  (import "M" "b" (memory $b 1))
  (func (export "test") (param i32) (result i32)
    (i32.store $a (i32.const 0) (local.get 0))
    (i32.load $b (i32.const 0))
  )
)
(assert_return (invoke "test" (i32.const 42)) (i32.const 42))
(assert_return (invoke "test" (i32.const 0xdeadbeef)) (i32.const 0xdeadbeef))

