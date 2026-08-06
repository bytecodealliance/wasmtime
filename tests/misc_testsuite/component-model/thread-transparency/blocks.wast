;;! component_model_async = true

;; $Inner's synchronously-lifted `async`-typed export really blocks, on a
;; `waitable-set.wait` over a fresh, empty waitable set. If the
;; thread-transparent $Outer -> $Mid adapter left `task_may_block` set, this
;; synchronous call chain would suspend instead of trapping.

(component definition $Tester
  (component $Inner
    (core module $Memory (memory (export "mem") 1))
    (core instance $memory (instantiate $Memory))
    (core module $M
      (import "" "waitable-set.new" (func $ws-new (result i32)))
      (import "" "waitable-set.wait" (func $ws-wait (param i32 i32) (result i32)))
      (func (export "g")
        (drop (call $ws-wait (call $ws-new) (i32.const 0)))
        unreachable)
    )
    (core func $ws-new (canon waitable-set.new))
    (core func $ws-wait
      (canon waitable-set.wait (memory (core memory $memory "mem"))))
    (core instance $m (instantiate $M
      (with "" (instance
        (export "waitable-set.new" (func $ws-new))
        (export "waitable-set.wait" (func $ws-wait))))))
    (func (export "g") async (canon lift (core func $m "g")))
  )

  (component $Mid
    (import "inner" (instance $inner (export "g" (func async))))
    (core module $M
      (import "" "g" (func $g))
      (func (export "f") (call $g))
    )
    (canon lower (func $inner "g") (core func $g))
    (core instance $m (instantiate $M
      (with "" (instance (export "g" (func $g))))))
    (func (export "f") (canon lift (core func $m "f")))
  )

  (component $Outer
    (import "mid" (instance $mid (export "f" (func))))
    (core module $M
      (import "" "task.return" (func $task-return))
      (import "" "f" (func $f))
      (func (export "run") (result i32)
        (call $f)
        (call $task-return)
        (i32.const 0 (; EXIT ;)))
      (func (export "cb") (param i32 i32 i32) (result i32) unreachable)
    )
    (canon task.return (core func $task-return))
    (canon lower (func $mid "f") (core func $f))
    (core instance $m (instantiate $M
      (with "" (instance
        (export "task.return" (func $task-return))
        (export "f" (func $f))))))
    (func (export "run") async
      (canon lift (core func $m "run") async (callback (core func $m "cb"))))
  )

  (instance $inner (instantiate $Inner))
  (instance $mid (instantiate $Mid (with "inner" (instance $inner))))
  (instance $outer (instantiate $Outer (with "mid" (instance $mid))))
  (func (export "run") (alias export $outer "run"))
)

(component instance $i $Tester)
(assert_trap (invoke "run") "cannot block a synchronous task before returning")
