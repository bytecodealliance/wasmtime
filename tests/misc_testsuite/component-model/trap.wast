;;! component_model_async = true

(component
  (component $Inner
    (core module $M
      (func (export "f") (result i32) unreachable)
    )
    (core instance $m (instantiate $M))
    (func (export "f") (result u32) (canon lift (core func $m "f")))
  )

  (component $Outer
    (import "f" (func $f (result u32)))
    (core func $f-lowered (canon lower (func $f)))
    (core module $N
      (import "" "f" (func $f (result i32)))
      (func (export "g") (result i32) (call $f))
    )
    (core instance $n (instantiate $N (with "" (instance
      (export "f" (func $f-lowered))
    ))))
    (func (export "g") (result u32) (canon lift (core func $n "g")))
  )

  (instance $inner (instantiate $Inner))
  (instance $outer (instantiate $Outer (with "f" (func $inner "f"))))
  (export "g" (func $outer "g"))
)

(assert_trap (invoke "g") "wasm `unreachable` instruction executed")

;; Lifted thunks that return and that trap.
(component
  (core module $m
    (func (export "thunk"))
    (func (export "thunk-trap") unreachable)
  )
  (core instance $i (instantiate $m))
  (func (export "thunk") (canon lift (core func $i "thunk")))
  (func (export "thunk-trap") (canon lift (core func $i "thunk-trap")))
)

(assert_return (invoke "thunk"))
(assert_trap (invoke "thunk-trap") "wasm `unreachable` instruction executed")
