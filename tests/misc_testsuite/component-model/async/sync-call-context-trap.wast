;;! component_model_async = true
;;! component_model_more_async_builtins = true

;; Test a guest-to-guest sync call whose callee traps. This should exercise
;; deferred task construction/teardown in the face of traps.
(component
  (component $A
    (core func $cset (canon context.set i32 0))
    (core module $M
      (import "" "context.set" (func $cset (param i32)))
      (func (export "f'") (param i32) (result i32)
        (call $cset (i32.const 0x5678))
        unreachable
      )
    )
    (core instance $m (instantiate $M (with "" (instance
      (export "context.set" (func $cset))
    ))))
    (func (export "f") (param "x" u32) (result u32)
      (canon lift (core func $m "f'")))
  )

  (component $B
    (import "f" (func $f (param "x" u32) (result u32)))
    (core func $f' (canon lower (func $f)))
    (core func $cset (canon context.set i32 0))
    (core module $N
      (import "" "f'" (func $f' (param i32) (result i32)))
      (import "" "context.set" (func $cset (param i32)))
      (func (export "g'") (result i32)
        (call $cset (i32.const 0x1234))
        (call $f' (i32.const 1234))
      )
    )
    (core instance $n (instantiate $N (with "" (instance
      (export "f'" (func $f'))
      (export "context.set" (func $cset))
    ))))
    (func (export "g") (result u32)
      (canon lift (core func $n "g'")))
  )

  (instance $a (instantiate $A))
  (instance $b (instantiate $B (with "f" (func $a "f"))))
  (export "g" (func $b "g"))
)
(assert_trap (invoke "g") "wasm `unreachable` instruction executed")
