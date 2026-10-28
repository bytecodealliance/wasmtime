;;! component_model_gc = true
;;! gc = true

;; Tests for the component model `bool` type under the GC data model, where
;; `bool` lowers to a core `i32` value.
;;
;; See https://github.com/WebAssembly/component-model/issues/525.

;; Both sides use the GC data model.
(component
  (component $A
    (core module $m
      (func (export "f") (param i32) (result i32) local.get 0)
    )
    (core instance $i (instantiate $m))
    (func (export "f") (param "a" bool) (result bool)
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

  (func (export "g") (param "a" bool) (result bool)
    (canon lift (core func $i "g") gc))
)

(assert_return (invoke "g" (bool.const false)) (bool.const false))
(assert_return (invoke "g" (bool.const true)) (bool.const true))

;; A GC-lowered caller calling a linear-memory-lifted callee.
(component
  (component $A
    (core module $m
      (memory (export "m") 1)
      (func (export "f") (param i32) (result i32) local.get 0)
    )
    (core instance $i (instantiate $m))
    (func (export "f") (param "a" bool) (result bool)
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

  (func (export "g") (param "a" bool) (result bool)
    (canon lift (core func $i "g") gc))
)

(assert_return (invoke "g" (bool.const false)) (bool.const false))
(assert_return (invoke "g" (bool.const true)) (bool.const true))

;; A linear-memory-lowered caller calling a GC-lifted callee.
(component
  (component $A
    (core module $m
      (func (export "f") (param i32) (result i32) local.get 0)
    )
    (core instance $i (instantiate $m))
    (func (export "f") (param "a" bool) (result bool)
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

  (func (export "g") (param "a" bool) (result bool)
    (canon lift (core func $i "g") (memory (core memory $i "m"))))
)

(assert_return (invoke "g" (bool.const false)) (bool.const false))
(assert_return (invoke "g" (bool.const true)) (bool.const true))

;; `bool` is canonicalized: a zero byte lifts to `false`.
(component
  (component $A
    (core module $m
      (func (export "f") (param i32) (result i32) i32.const 0)
    )
    (core instance $i (instantiate $m))
    (func (export "f") (param "a" bool) (result bool)
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

  (func (export "g") (param "a" bool) (result bool)
    (canon lift (core func $i "g") gc))
)

(assert_return (invoke "g" (bool.const false)) (bool.const false))

;; Any non-zero value lifts to `true`, not just 1.
(component
  (component $A
    (core module $m
      (func (export "f") (param i32) (result i32) i32.const 0x2)
    )
    (core instance $i (instantiate $m))
    (func (export "f") (param "a" bool) (result bool)
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

  (func (export "g") (param "a" bool) (result bool)
    (canon lift (core func $i "g") gc))
)

(assert_return (invoke "g" (bool.const false)) (bool.const true))

;; All-ones likewise lifts to `true`.
(component
  (component $A
    (core module $m
      (func (export "f") (param i32) (result i32) i32.const 0xffffffff)
    )
    (core instance $i (instantiate $m))
    (func (export "f") (param "a" bool) (result bool)
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

  (func (export "g") (param "a" bool) (result bool)
    (canon lift (core func $i "g") gc))
)

(assert_return (invoke "g" (bool.const false)) (bool.const true))
