;;! component_model_async = true
;;! component_model_more_async_builtins = true
;;! component_model_async_stackful = true
;;! component_model_threading = true

;; Sync-to-sync adapters between sibling instances have no dynamic `may_enter`
;; check, so a guest that cares about reentrance has to guard against it itself.
;; This exercises the case where it actually happens:
;;
;;   host --async-lift--> $Outer --adapter--> $Mid --async-typed--> $Inner
;;
;; $Inner blocks on a fresh, empty waitable set. Blocking in a sync-typed call
;; no longer traps outright: the scheduler first looks for another eligible
;; thread to run, and $Outer has started a second one. That second thread calls
;; $Mid.f while the first call's frame is still live, reentrance now being
;; permitted, so $Mid's own `$inside` guard is what fires.
;;
;; This is not an artifact of dropping the `{enter,exit}-sync-call` window: the
;; $Outer -> $Mid adapter is opaque here anyway, because $Mid lowers an
;; `async`-typed lift.

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

  ;; $Mid traps if it is ever re-entered while a previous call is still in
  ;; progress.
  (component $Mid
    (import "inner" (instance $inner (export "g" (func async))))
    (core module $M
      (import "" "g" (func $g))
      (global $inside (mut i32) (i32.const 0))
      (func (export "f")
        (if (global.get $inside)
          (then unreachable (; REENTERED ;)))
        (global.set $inside (i32.const 1))
        (call $g)
        (global.set $inside (i32.const 0)))
    )
    (canon lower (func $inner "g") (core func $g))
    (core instance $m (instantiate $M
      (with "" (instance (export "g" (func $g))))))
    (func (export "f") (canon lift (core func $m "f")))
  )

  (component $Outer
    (import "mid" (instance $mid (export "f" (func))))
    (core module $Table
      (table (export "__indirect_function_table") 1 funcref))
    (core instance $table (instantiate $Table))
    (core module $M
      (import "" "task.return" (func $task-return))
      (import "" "f" (func $f))
      (import "" "thread.new-indirect" (func $thread-new (param i32 i32) (result i32)))
      (import "" "thread.resume-later" (func $resume-later (param i32)))
      (import "" "__indirect_function_table" (table $tbl 1 funcref))

      ;; Runs on the second thread, once the first one has blocked.
      (func $second (param i32)
        (call $f))
      (elem (table $tbl) (i32.const 0) func $second)

      (func (export "run") (result i32)
        (call $resume-later (call $thread-new (i32.const 0) (i32.const 0)))
        (call $f)
        (call $task-return)
        (i32.const 0 (; EXIT ;)))
      (func (export "cb") (param i32 i32 i32) (result i32) unreachable)
    )
    (core type $start-func-ty (func (param i32)))
    (alias core export $table "__indirect_function_table"
      (core table $indirect-function-table))
    (core func $thread-new
      (canon thread.new-indirect $start-func-ty
        (core table $indirect-function-table)))
    (core func $resume-later (canon thread.resume-later))
    (canon task.return (core func $task-return))
    (canon lower (func $mid "f") (core func $f))
    (core instance $m (instantiate $M
      (with "" (instance
        (export "task.return" (func $task-return))
        (export "f" (func $f))
        (export "thread.new-indirect" (func $thread-new))
        (export "thread.resume-later" (func $resume-later))
        (export "__indirect_function_table" (table $indirect-function-table))))))
    (func (export "run") async
      (canon lift (core func $m "run") async (callback (core func $m "cb"))))
  )

  (instance $inner (instantiate $Inner))
  (instance $mid (instantiate $Mid (with "inner" (instance $inner))))
  (instance $outer (instantiate $Outer (with "mid" (instance $mid))))
  (func (export "run") (alias export $outer "run"))
)

(component instance $i $Tester)
(assert_trap (invoke "run") "wasm `unreachable` instruction executed")
