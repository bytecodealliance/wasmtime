;;! component_model_async = true

;; The transparency analysis works at component-instance, rather than
;; core-instance, granularity: $Inner's lifted callee imports nothing but a core
;; table, but a sibling core instance planted a `context`-poking function in it
;; that `call_indirect` reaches. Since $Inner declares `context.{get,set}`, the
;; $Outer -> $Inner adapter is opaque and keeps its `{enter,exit}-sync-call`
;; calls, so the value $Evil writes cannot land in $Outer's slot.

(component
  (component $Inner
    (core func $cget (canon context.get i32 0))
    (core func $cset (canon context.set i32 0))

    (core module $Shared (table (export "t") 1 funcref))
    (core instance $shared (instantiate $Shared))

    (core module $Evil
      (import "" "t" (table 1 funcref))
      (import "" "cget" (func $cget (result i32)))
      (import "" "cset" (func $cset (param i32)))
      (func $leak (result i32)
        (call $cset (i32.const 0x5555))
        (call $cget))
      (elem (table 0) (i32.const 0) func $leak)
    )
    (core instance $evil (instantiate $Evil (with "" (instance
      (export "t" (table $shared "t"))
      (export "cget" (func $cget))
      (export "cset" (func $cset))))))

    (core module $Victim
      (import "" "t" (table 1 funcref))
      (type $sig (func (result i32)))
      (func (export "f") (result i32)
        (call_indirect (type $sig) (i32.const 0)))
    )
    (core instance $victim (instantiate $Victim (with "" (instance
      (export "t" (table $shared "t"))))))

    (func (export "f") (result u32) (canon lift (core func $victim "f")))
  )

  (component $Outer
    (import "f" (func $f (result u32)))
    (core func $f' (canon lower (func $f)))
    (core func $get (canon context.get i32 0))
    (core func $set (canon context.set i32 0))
    (core module $M
      (import "" "f" (func $f (result i32)))
      (import "" "get" (func $get (result i32)))
      (import "" "set" (func $set (param i32)))
      (func (export "g") (result i32) (local $r i32)
        (call $set (i32.const 0x1234abcd))
        (local.set $r (call $f))
        ;; $Evil ran and saw its own write; ours must be untouched.
        (if (i32.ne (local.get $r) (i32.const 0x5555)) (then unreachable))
        (if (i32.ne (call $get) (i32.const 0x1234abcd)) (then unreachable))
        (local.get $r))
    )
    (core instance $m (instantiate $M (with "" (instance
      (export "f" (func $f'))
      (export "get" (func $get))
      (export "set" (func $set))))))
    (func (export "g") (result u32) (canon lift (core func $m "g")))
  )

  (instance $inner (instantiate $Inner))
  (instance $outer (instantiate $Outer (with "f" (func $inner "f"))))
  (export "g" (func $outer "g"))
)
(assert_return (invoke "g") (u32.const 0x5555))
