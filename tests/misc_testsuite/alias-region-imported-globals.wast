;; Test that two imports of the same global correctly alias.
;; The alias region system should ensure that global.set through one import
;; is visible through global.get on the other import.

(module $M
  (global (export "a") (export "b") (mut i32) (i32.const 0))
)
(register "M")

(module
  (import "M" "a" (global $a (mut i32)))
  (import "M" "b" (global $b (mut i32)))
  (func (export "test") (param i32) (result i32)
    (global.set $a (local.get 0))
    (global.get $b)
  )
)
(assert_return (invoke "test" (i32.const 42)) (i32.const 42))
(assert_return (invoke "test" (i32.const 0xdeadbeef)) (i32.const 0xdeadbeef))
