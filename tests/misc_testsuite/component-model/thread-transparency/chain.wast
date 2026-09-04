;;! component_model_async = true

;; Opacity does not propagate in either direction along a call chain: each
;; adapter is judged purely on its own lift instance. Every case below is the
;; same $Outer -> $Mid -> $Inner chain with a different link made opaque by a
;; `context` canon, with $Outer driving a slot across the whole chain and
;; checking that it survived.

;; Opaque innermost link, transparent outer one.
(component
  (component $Inner
    (core func $get (canon context.get i32 0))
    (core func $set (canon context.set i32 0))
    (core module $M
      (import "" "get" (func $get (result i32)))
      (import "" "set" (func $set (param i32)))
      (func (export "f") (param i32) (result i32)
        (if (call $get) (then unreachable))
        (call $set (i32.const 0x5555ffff))
        (i32.add (local.get 0) (i32.const 2)))
    )
    (core instance $m (instantiate $M (with "" (instance
      (export "get" (func $get))
      (export "set" (func $set))))))
    (func (export "f") (param "x" u32) (result u32)
      (canon lift (core func $m "f")))
  )

  (component $Mid
    (import "f" (func $f (param "x" u32) (result u32)))
    (core func $f' (canon lower (func $f)))
    (core module $M
      (import "" "f" (func $f (param i32) (result i32)))
      (func (export "f") (param i32) (result i32)
        (i32.add (call $f (local.get 0)) (i32.const 20)))
    )
    (core instance $m (instantiate $M
      (with "" (instance (export "f" (func $f'))))))
    (func (export "f") (param "x" u32) (result u32)
      (canon lift (core func $m "f")))
  )

  (component $Outer
    (import "f" (func $f (param "x" u32) (result u32)))
    (core func $f' (canon lower (func $f)))
    (core func $get (canon context.get i32 0))
    (core func $set (canon context.set i32 0))
    (core module $M
      (import "" "f" (func $f (param i32) (result i32)))
      (import "" "get" (func $get (result i32)))
      (import "" "set" (func $set (param i32)))
      (func (export "g") (result i32) (local $r i32)
        (call $set (i32.const 0x1234abcd))
        (local.set $r (call $f (i32.const 20)))
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
  (instance $mid (instantiate $Mid (with "f" (func $inner "f"))))
  (instance $outer (instantiate $Outer (with "f" (func $mid "f"))))
  (export "g" (func $outer "g"))
)
(assert_return (invoke "g") (u32.const 42))

;; Transparent innermost link, opaque outer one.
(component
  (component $Inner
    (core module $M
      (func (export "f") (param i32) (result i32)
        (i32.add (local.get 0) (i32.const 2)))
    )
    (core instance $m (instantiate $M))
    (func (export "f") (param "x" u32) (result u32)
      (canon lift (core func $m "f")))
  )

  (component $Mid
    (import "f" (func $f (param "x" u32) (result u32)))
    (core func $f' (canon lower (func $f)))
    (core func $get (canon context.get i32 0))
    (core func $set (canon context.set i32 0))
    (core module $M
      (import "" "f" (func $f (param i32) (result i32)))
      (import "" "get" (func $get (result i32)))
      (import "" "set" (func $set (param i32)))
      (func (export "f") (param i32) (result i32) (local $r i32)
        (if (call $get) (then unreachable))
        (call $set (i32.const 0x5555ffff))
        (local.set $r (call $f (local.get 0)))
        ;; The transparent callee below cannot have disturbed our slot.
        (if (i32.ne (call $get) (i32.const 0x5555ffff)) (then unreachable))
        (i32.add (local.get $r) (i32.const 20)))
    )
    (core instance $m (instantiate $M (with "" (instance
      (export "f" (func $f'))
      (export "get" (func $get))
      (export "set" (func $set))))))
    (func (export "f") (param "x" u32) (result u32)
      (canon lift (core func $m "f")))
  )

  (component $Outer
    (import "f" (func $f (param "x" u32) (result u32)))
    (core func $f' (canon lower (func $f)))
    (core func $get (canon context.get i32 0))
    (core func $set (canon context.set i32 0))
    (core module $M
      (import "" "f" (func $f (param i32) (result i32)))
      (import "" "get" (func $get (result i32)))
      (import "" "set" (func $set (param i32)))
      (func (export "g") (result i32) (local $r i32)
        (call $set (i32.const 0x1234abcd))
        (local.set $r (call $f (i32.const 20)))
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
  (instance $mid (instantiate $Mid (with "f" (func $inner "f"))))
  (instance $outer (instantiate $Outer (with "f" (func $mid "f"))))
  (export "g" (func $outer "g"))
)
(assert_return (invoke "g") (u32.const 42))

;; Both links opaque, each with its own slot value to keep straight.
(component
  (component $Inner
    (core func $get (canon context.get i32 0))
    (core func $set (canon context.set i32 0))
    (core module $M
      (import "" "get" (func $get (result i32)))
      (import "" "set" (func $set (param i32)))
      (func (export "f") (param i32) (result i32)
        (if (call $get) (then unreachable))
        (call $set (i32.const 0x7777ffff))
        (i32.add (local.get 0) (i32.const 2)))
    )
    (core instance $m (instantiate $M (with "" (instance
      (export "get" (func $get))
      (export "set" (func $set))))))
    (func (export "f") (param "x" u32) (result u32)
      (canon lift (core func $m "f")))
  )

  (component $Mid
    (import "f" (func $f (param "x" u32) (result u32)))
    (core func $f' (canon lower (func $f)))
    (core func $get (canon context.get i32 0))
    (core func $set (canon context.set i32 0))
    (core module $M
      (import "" "f" (func $f (param i32) (result i32)))
      (import "" "get" (func $get (result i32)))
      (import "" "set" (func $set (param i32)))
      (func (export "f") (param i32) (result i32) (local $r i32)
        (if (call $get) (then unreachable))
        (call $set (i32.const 0x5555ffff))
        (local.set $r (call $f (local.get 0)))
        (if (i32.ne (call $get) (i32.const 0x5555ffff)) (then unreachable))
        (i32.add (local.get $r) (i32.const 20)))
    )
    (core instance $m (instantiate $M (with "" (instance
      (export "f" (func $f'))
      (export "get" (func $get))
      (export "set" (func $set))))))
    (func (export "f") (param "x" u32) (result u32)
      (canon lift (core func $m "f")))
  )

  (component $Outer
    (import "f" (func $f (param "x" u32) (result u32)))
    (core func $f' (canon lower (func $f)))
    (core func $get (canon context.get i32 0))
    (core func $set (canon context.set i32 0))
    (core module $M
      (import "" "f" (func $f (param i32) (result i32)))
      (import "" "get" (func $get (result i32)))
      (import "" "set" (func $set (param i32)))
      (func (export "g") (result i32) (local $r i32)
        (call $set (i32.const 0x1234abcd))
        (local.set $r (call $f (i32.const 20)))
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
  (instance $mid (instantiate $Mid (with "f" (func $inner "f"))))
  (instance $outer (instantiate $Outer (with "f" (func $mid "f"))))
  (export "g" (func $outer "g"))
)
(assert_return (invoke "g") (u32.const 42))
