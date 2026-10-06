;;! component_model_async = true

;; `task.return` and `task.cancel` may only be called from tasks created by an
;; `async`-lifted export; calling them from a sync-lifted export traps.

;; A sync-lifted export calls `task.return`, called from the host.
(component
  (core func $task.return (canon task.return))
  (core module $m
    (import "" "task.return" (func $task.return))
    (func (export "f")
      (call $task.return)))
  (core instance $i (instantiate $m
    (with "" (instance (export "task.return" (func $task.return))))))
  (func (export "f") async (canon lift (core func $i "f")))
)
(assert_trap (invoke "f") "not lifted with `async`")

;; A sync-lifted export calls `task.return`, called through an async-lowered
;; import.
(component
  (component $A
    (core func $task.return (canon task.return))
    (core module $a
      (import "" "task.return" (func $task.return))
      (func (export "f")
        (call $task.return)))
    (core instance $a (instantiate $a
      (with "" (instance (export "task.return" (func $task.return))))))
    (func (export "f") async (canon lift (core func $a "f")))
  )
  (component $B
    (import "a" (instance $a (export "f" (func async))))
    (core func $f (canon lower (func $a "f") async))
    (core module $b
      (import "" "f" (func $f (result i32)))
      (func (export "run") (drop (call $f))))
    (core instance $b (instantiate $b
      (with "" (instance (export "f" (func $f))))))
    (func (export "run") (canon lift (core func $b "run")))
  )
  (instance $a (instantiate $A))
  (instance $b (instantiate $B (with "a" (instance $a))))
  (export "run" (func $b "run"))
)
(assert_trap (invoke "run") "not lifted with `async`")

;; A sync-lifted export calls `task.return`, called through a sync-lowered
;; import from an async-lifted (callback) export.
(component
  (component $A
    (core func $task.return (canon task.return))
    (core module $a
      (import "" "task.return" (func $task.return))
      (func (export "f")
        (call $task.return)))
    (core instance $a (instantiate $a
      (with "" (instance (export "task.return" (func $task.return))))))
    (func (export "f") (canon lift (core func $a "f")))
  )
  (component $B
    (import "a" (instance $a (export "f" (func))))
    (core func $f (canon lower (func $a "f")))
    (core module $b
      (import "" "f" (func $f))
      (func (export "run") (result i32) (call $f) (i32.const 0 (; EXIT ;)))
      (func (export "cb") (param i32 i32 i32) (result i32) unreachable))
    (core instance $b (instantiate $b
      (with "" (instance (export "f" (func $f))))))
    (func (export "run") async
      (canon lift (core func $b "run") async (callback (core func $b "cb"))))
  )
  (instance $a (instantiate $A))
  (instance $b (instantiate $B (with "a" (instance $a))))
  (export "run" (func $b "run"))
)
(assert_trap (invoke "run") "not lifted with `async`")

;; A sync-lifted export calls `task.cancel`.
(component
  (core func $task.cancel (canon task.cancel))
  (core module $m
    (import "" "task.cancel" (func $task.cancel))
    (func (export "f")
      (call $task.cancel)))
  (core instance $i (instantiate $m
    (with "" (instance (export "task.cancel" (func $task.cancel))))))
  (func (export "f") async (canon lift (core func $i "f")))
)
(assert_trap (invoke "f") "not lifted with `async`")
