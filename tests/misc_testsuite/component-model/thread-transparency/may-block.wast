;;! component_model_async = true

;; A sync-typed call reaching an `async`-typed export, through a chain that
;; includes a guest-to-guest adapter:
;;
;;   host --async-lift--> $Outer --adapter--> $Mid --async-typed--> $Inner
;;
;; Blocking in a sync-typed call is enforced lazily: making the call is allowed,
;; and only actually blocking traps (see `blocks.wast`, where $Inner really does
;; block). $Inner's body here never blocks, so the call goes through and its
;; `unreachable` is what surfaces.
;;
;; The point of this file is that that outcome does not depend on how the
;; $Outer -> $Mid adapter is classified: `opaque-mid.wast` is this same
;; component with one extra `canon context.get` in $Mid, which forces the
;; adapter to keep its `{enter,exit}-sync-call` window, and asserts the same
;; result.

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

  ;; $Mid declares no canonical built-in beyond a `canon lower` of an imported
  ;; lift. That lift is `async`-typed, which is itself disqualifying, so the
  ;; $Outer -> $Mid adapter keeps its window here; `opaque-mid.wast` is the same
  ;; component with a second, independent reason to keep it.
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
