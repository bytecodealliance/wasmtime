;;! component_model_async = true
;;! reference_types = true

;; An example component where a sync lower calls an async lift. The async lift
;; is an early-return but then keeps going afterwards for a late exit.

(component
  (component $A
    (core module $m
      (import "" "task.return" (func $task.return (param i32)))
      (func (export "callback") (param i32 i32 i32) (result i32)
        i32.const 0 ;; EXIT
      )
      (func (export "foo") (param i32) (result i32)
        (call $task.return (local.get 0))
        i32.const 1 ;; YIELD
      )
    )
    (core func $task.return (canon task.return (result u32)))
    (core instance $i (instantiate $m
      (with "" (instance (export "task.return" (func $task.return))))
    ))

    (func (export "foo") async (param "p1" u32) (result u32)
      (canon lift (core func $i "foo") async (callback (core func $i "callback")))
    )
  )

  (component $B
    (import "a" (func $foo async (param "p1" u32) (result u32)))
    (core func $foo (canon lower (func $foo)))
    (core module $m
      (import "" "foo" (func $foo (param i32) (result i32)))
      (func (export "run")
        (i32.ne (call $foo (i32.const 42)) (i32.const 42))
        if unreachable end
      )
    )
    (core instance $i (instantiate $m
      (with "" (instance (export "foo" (func $foo))))
    ))
    (func (export "run") (canon lift (core func $i "run")))
  )

  (instance $A (instantiate $A))
  (instance $B (instantiate $B (with "a" (func $A "foo"))))
  (func (export "run") (alias export $B "run"))
)

(assert_return (invoke "run"))

;; A trap in an early-returning task's callback, while another task's start is
;; queued behind it.
;; - `a` calls `task.return`, then makes a future ready and WAITs on a set
;;   holding it, which queues a callback invocation in the event loop.
;; - `b`'s start is queued behind that callback, which traps, and the trap
;;   surfaces from `b`'s call.
;; Dropping a typed `call_async` future in this state is tested in Rust by
;; `drop_call_async_with_queued_start`.
(component
  (type $f (future))
  (core func $future.new (canon future.new $f))
  (core func $future.read (canon future.read $f async))
  (core func $future.write (canon future.write $f async))
  (core func $waitable-set.new (canon waitable-set.new))
  (core func $waitable.join (canon waitable.join))
  (core func $task.return (canon task.return))

  (core module $m
    (import "" "future.new" (func $future.new (result i64)))
    (import "" "future.read" (func $future.read (param i32 i32) (result i32)))
    (import "" "future.write"
      (func $future.write (param i32 i32) (result i32)))
    (import "" "waitable-set.new" (func $waitable-set.new (result i32)))
    (import "" "waitable.join" (func $waitable.join (param i32 i32)))
    (import "" "task.return" (func $task.return))

    (func (export "a") (result i32)
      (local $t i64) (local $r i32) (local $w i32) (local $s i32)
      (call $task.return)
      (local.set $t (call $future.new))
      (local.set $r (i32.wrap_i64 (local.get $t)))
      (local.set $w (i32.wrap_i64 (i64.shr_u (local.get $t) (i64.const 32))))
      (drop (call $future.read (local.get $r) (i32.const 0)))
      (local.set $s (call $waitable-set.new))
      (call $waitable.join (local.get $r) (local.get $s))
      (drop (call $future.write (local.get $w) (i32.const 0)))
      ;; WAIT on a set which already has a ready event.
      (i32.or (i32.const 2 (; WAIT ;)) (i32.shl (local.get $s) (i32.const 4)))
    )
    (func (export "callback") (param i32 i32 i32) (result i32) unreachable)
    (func (export "b") (result i32)
      (call $task.return)
      (i32.const 0 (; EXIT ;))
    )
  )
  (core instance $i (instantiate $m
    (with "" (instance
      (export "future.new" (func $future.new))
      (export "future.read" (func $future.read))
      (export "future.write" (func $future.write))
      (export "waitable-set.new" (func $waitable-set.new))
      (export "waitable.join" (func $waitable.join))
      (export "task.return" (func $task.return))
    ))
  ))

  (func (export "a") async
    (canon lift (core func $i "a") async (callback (core func $i "callback"))))
  (func (export "b") async
    (canon lift (core func $i "b") async (callback (core func $i "callback"))))
)
(assert_return (invoke "a"))
(assert_trap (invoke "b") "unreachable")
