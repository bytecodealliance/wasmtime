;;! exceptions = true

;; A small test which ensures that `catch_all` exception handlers are not
;; executed for normal traps. This test invokes some functions which trap and
;; asserts that none of them run exception handlers. It then runs one function
;; that throws and asserts that does indeed run the exception handler.

(module
  (global $g (mut i32) (i32.const 0))
  (table 10 funcref)
  (tag $t)
  (elem (i32.const 0) func
    $unreachable
    $div-by-zero
    $stack-overflow
    $actually-throw
  )

  (func $unreachable unreachable)
  (func $div-by-zero i32.const 1 i32.const 0 i32.div_s drop)
  (func $stack-overflow call $stack-overflow)
  (func $actually-throw throw $t)

  (func (export "run") (param i32)
    (global.set $g (i32.const 0))
    (block $h
      (try_table (catch_all $h)
        (call_indirect (local.get 0))
        return
      )
    )
    (global.set $g (i32.const 1))
  )

  (func (export "g") (result i32) (global.get $g))
)


(assert_trap (invoke "run" (i32.const 0)) "unreachable")
(assert_return (invoke "g") (i32.const 0))

(assert_trap (invoke "run" (i32.const 1)) "divide by zero")
(assert_return (invoke "g") (i32.const 0))

(assert_trap (invoke "run" (i32.const 2)) "call stack exhausted")
(assert_return (invoke "g") (i32.const 0))

(assert_return (invoke "run" (i32.const 3)))
(assert_return (invoke "g") (i32.const 1))

;; make sure there's no stale state or anything like that
(assert_trap (invoke "run" (i32.const 0)) "unreachable")
(assert_return (invoke "g") (i32.const 0))
