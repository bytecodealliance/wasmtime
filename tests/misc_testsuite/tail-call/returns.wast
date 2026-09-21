;; Ordinary returns with no stack arguments and with adjacent local-frame sizes.
;; On x86-64 the last two functions straddle Winch's compact epilogue boundary.

(module
  (func (export "no_stack_args") (result i64)
    i64.const 42)
  (func (export "compact") (param i64 i64 i64 i64 i64 i64 i64 i64) (result i64)
    (local i64 i64 i64 i64)
    local.get 0 local.get 7 i64.add)
  (func (export "fallback") (param i64 i64 i64 i64 i64 i64 i64 i64) (result i64)
    (local i64 i64 i64 i64 i64)
    local.get 0 local.get 7 i64.add))

(assert_return (invoke "no_stack_args") (i64.const 42))

(assert_return
  (invoke "compact" (i64.const -17) (i64.const 2) (i64.const 3) (i64.const 4)
    (i64.const 5) (i64.const 6) (i64.const 7) (i64.const -9))
  (i64.const -26))
(assert_return (invoke "no_stack_args") (i64.const 42))

(assert_return
  (invoke "compact" (i64.const 0) (i64.const 2) (i64.const 3) (i64.const 4)
    (i64.const 5) (i64.const 6) (i64.const 7) (i64.const 8))
  (i64.const 8))
(assert_return (invoke "no_stack_args") (i64.const 42))

(assert_return
  (invoke "compact" (i64.const 42) (i64.const 2) (i64.const 3) (i64.const 4)
    (i64.const 5) (i64.const 6) (i64.const 7) (i64.const 50))
  (i64.const 92))
(assert_return (invoke "no_stack_args") (i64.const 42))

(assert_return
  (invoke "fallback" (i64.const -17) (i64.const 2) (i64.const 3) (i64.const 4)
    (i64.const 5) (i64.const 6) (i64.const 7) (i64.const -9))
  (i64.const -26))
(assert_return (invoke "no_stack_args") (i64.const 42))

(assert_return
  (invoke "fallback" (i64.const 0) (i64.const 2) (i64.const 3) (i64.const 4)
    (i64.const 5) (i64.const 6) (i64.const 7) (i64.const 8))
  (i64.const 8))
(assert_return (invoke "no_stack_args") (i64.const 42))

(assert_return
  (invoke "fallback" (i64.const 42) (i64.const 2) (i64.const 3) (i64.const 4)
    (i64.const 5) (i64.const 6) (i64.const 7) (i64.const 50))
  (i64.const 92))
(assert_return (invoke "no_stack_args") (i64.const 42))
