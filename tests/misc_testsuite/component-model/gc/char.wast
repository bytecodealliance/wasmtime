;;! component_model_gc = true
;;! gc = true

;; Tests for the component model `char` type under the GC data model, where
;; `char` lowers to a core `i32` value.
;;
;; See https://github.com/WebAssembly/component-model/issues/525.

;; Both sides use the GC data model.
(component
  (component $A
    (core module $m
      (func (export "f") (param i32) (result i32) local.get 0)
    )
    (core instance $i (instantiate $m))
    (func (export "f") (param "a" char) (result char)
      (canon lift (core func $i "f") gc))
  )
  (instance $a (instantiate $A))
  (alias export $a "f" (func $af))

  (core type $sig (func (param i32) (result i32)))
  (core func $l (canon lower (func $af) gc (core-type $sig)))

  (core module $m
    (import "" "f" (func $f (param i32) (result i32)))
    (func (export "g") (param i32) (result i32) local.get 0 call $f)
  )
  (core instance $i (instantiate $m (with "" (instance (export "f" (func $l))))))

  (func (export "g") (param "a" char) (result char)
    (canon lift (core func $i "g") gc))
)

(assert_return (invoke "g" (char.const "a")) (char.const "a"))
(assert_return (invoke "g" (char.const "\u{0}")) (char.const "\u{0}"))
(assert_return (invoke "g" (char.const "\u{d7ff}")) (char.const "\u{d7ff}"))
(assert_return (invoke "g" (char.const "\u{e000}")) (char.const "\u{e000}"))
(assert_return (invoke "g" (char.const "\u{10ffff}")) (char.const "\u{10ffff}"))
(assert_return (invoke "g" (char.const "🍰")) (char.const "🍰"))

;; A GC-lowered caller calling a linear-memory-lifted callee.
(component
  (component $A
    (core module $m
      (memory (export "m") 1)
      (func (export "f") (param i32) (result i32) local.get 0)
    )
    (core instance $i (instantiate $m))
    (func (export "f") (param "a" char) (result char)
      (canon lift (core func $i "f") (memory (core memory $i "m"))))
  )
  (instance $a (instantiate $A))
  (alias export $a "f" (func $af))

  (core type $sig (func (param i32) (result i32)))
  (core func $l (canon lower (func $af) gc (core-type $sig)))

  (core module $m
    (import "" "f" (func $f (param i32) (result i32)))
    (func (export "g") (param i32) (result i32) local.get 0 call $f)
  )
  (core instance $i (instantiate $m (with "" (instance (export "f" (func $l))))))

  (func (export "g") (param "a" char) (result char)
    (canon lift (core func $i "g") gc))
)

(assert_return (invoke "g" (char.const "a")) (char.const "a"))
(assert_return (invoke "g" (char.const "\u{0}")) (char.const "\u{0}"))
(assert_return (invoke "g" (char.const "\u{d7ff}")) (char.const "\u{d7ff}"))
(assert_return (invoke "g" (char.const "\u{e000}")) (char.const "\u{e000}"))
(assert_return (invoke "g" (char.const "\u{10ffff}")) (char.const "\u{10ffff}"))
(assert_return (invoke "g" (char.const "🍰")) (char.const "🍰"))

;; A linear-memory-lowered caller calling a GC-lifted callee.
(component
  (component $A
    (core module $m
      (func (export "f") (param i32) (result i32) local.get 0)
    )
    (core instance $i (instantiate $m))
    (func (export "f") (param "a" char) (result char)
      (canon lift (core func $i "f") gc))
  )
  (instance $a (instantiate $A))
  (alias export $a "f" (func $af))

  (core func $l (canon lower (func $af)))

  (core module $m
    (import "" "f" (func $f (param i32) (result i32)))
    (memory (export "m") 1)
    (func (export "g") (param i32) (result i32) local.get 0 call $f)
  )
  (core instance $i (instantiate $m (with "" (instance (export "f" (func $l))))))

  (func (export "g") (param "a" char) (result char)
    (canon lift (core func $i "g") (memory (core memory $i "m"))))
)

(assert_return (invoke "g" (char.const "a")) (char.const "a"))
(assert_return (invoke "g" (char.const "\u{0}")) (char.const "\u{0}"))
(assert_return (invoke "g" (char.const "\u{d7ff}")) (char.const "\u{d7ff}"))
(assert_return (invoke "g" (char.const "\u{e000}")) (char.const "\u{e000}"))
(assert_return (invoke "g" (char.const "\u{10ffff}")) (char.const "\u{10ffff}"))
(assert_return (invoke "g" (char.const "🍰")) (char.const "🍰"))

;; A leading surrogate is not a valid `char`.
(component
  (component $A
    (core module $m
      (func (export "f") (param i32) (result i32) i32.const 0xd800)
    )
    (core instance $i (instantiate $m))
    (func (export "f") (param "a" char) (result char)
      (canon lift (core func $i "f") gc))
  )
  (instance $a (instantiate $A))
  (alias export $a "f" (func $af))

  (core type $sig (func (param i32) (result i32)))
  (core func $l (canon lower (func $af) gc (core-type $sig)))

  (core module $m
    (import "" "f" (func $f (param i32) (result i32)))
    (func (export "g") (param i32) (result i32) local.get 0 call $f)
  )
  (core instance $i (instantiate $m (with "" (instance (export "f" (func $l))))))

  (func (export "g") (param "a" char) (result char)
    (canon lift (core func $i "g") gc))
)

(assert_trap (invoke "g" (char.const "a")) "invalid `char` bit pattern")

;; Nor is a trailing surrogate.
(component
  (component $A
    (core module $m
      (func (export "f") (param i32) (result i32) i32.const 0xdfff)
    )
    (core instance $i (instantiate $m))
    (func (export "f") (param "a" char) (result char)
      (canon lift (core func $i "f") gc))
  )
  (instance $a (instantiate $A))
  (alias export $a "f" (func $af))

  (core type $sig (func (param i32) (result i32)))
  (core func $l (canon lower (func $af) gc (core-type $sig)))

  (core module $m
    (import "" "f" (func $f (param i32) (result i32)))
    (func (export "g") (param i32) (result i32) local.get 0 call $f)
  )
  (core instance $i (instantiate $m (with "" (instance (export "f" (func $l))))))

  (func (export "g") (param "a" char) (result char)
    (canon lift (core func $i "g") gc))
)

(assert_trap (invoke "g" (char.const "a")) "invalid `char` bit pattern")

;; Nor is a value at or above the maximum code point.
(component
  (component $A
    (core module $m
      (func (export "f") (param i32) (result i32) i32.const 0x110000)
    )
    (core instance $i (instantiate $m))
    (func (export "f") (param "a" char) (result char)
      (canon lift (core func $i "f") gc))
  )
  (instance $a (instantiate $A))
  (alias export $a "f" (func $af))

  (core type $sig (func (param i32) (result i32)))
  (core func $l (canon lower (func $af) gc (core-type $sig)))

  (core module $m
    (import "" "f" (func $f (param i32) (result i32)))
    (func (export "g") (param i32) (result i32) local.get 0 call $f)
  )
  (core instance $i (instantiate $m (with "" (instance (export "f" (func $l))))))

  (func (export "g") (param "a" char) (result char)
    (canon lift (core func $i "g") gc))
)

(assert_trap (invoke "g" (char.const "a")) "invalid `char` bit pattern")
