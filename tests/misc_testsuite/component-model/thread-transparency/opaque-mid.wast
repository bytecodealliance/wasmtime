;;! component_model_async = true

;; Like `may-block.wast` except that $Mid declares one extra `canon
;; context.get`, which makes it opaque on its own account, so the $Outer -> $Mid
;; adapter keeps its `{enter,exit}-sync-call` calls. The result must be the same
;; as `may-block.wast`'s: how the adapter is classified is not observable.

(component definition $Tester
  ;; $Inner's `async`-typed export is lifted *synchronously*. Its body traps
  ;; rather than blocking, so the sync-typed call into it is permitted and this
  ;; `unreachable` is what the caller observes.
  (component $Inner
    (core module $M
      (func (export "g") unreachable)
    )
    (core instance $m (instantiate $M))
    (func (export "g") async (canon lift (core func $m "g")))
  )

  ;; $Mid would otherwise be thread-transparent: it declares no canonical
  ;; built-in beyond a `canon lower` of an imported lift, and its own export `f`
  ;; is sync on both sides and mentions no handles.
  (component $Mid
    (import "inner" (instance $inner (export "g" (func async))))
    (core module $M
      (import "" "g" (func $g))
      (func (export "f") (call $g))
    )
    (canon lower (func $inner "g") (core func $g))
    ;; The only difference from `may-block.wast`:
    ;; this canon makes $Mid opaque.
    (canon context.get i32 0 (core func $ctx))
    (core instance $m (instantiate $M
      (with "" (instance (export "g" (func $g))))))
    (func (export "f") (canon lift (core func $m "f")))
  )

  ;; $Outer's export is async-lifted, so its task is an "async function" and is
  ;; itself allowed to block while it runs.
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
(assert_trap (invoke "run") "wasm `unreachable` instruction executed")
