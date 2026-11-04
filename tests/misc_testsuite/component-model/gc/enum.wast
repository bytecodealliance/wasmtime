;;! component_model_gc = true
;;! gc = true

;; Tests for the component model `enum` type under the GC data model, where
;; `enum` lowers to a core `i32` value.
;;
;; See https://github.com/WebAssembly/component-model/issues/525.

;; Both sides use the GC data model.
(component
  (type $e (enum "red" "green" "blue"))
  (component $A
    (import "t" (type $t (eq $e)))
    (core module $m
      (func (export "f") (param i32) (result i32) local.get 0)
    )
    (core instance $i (instantiate $m))
    (func (export "f") (param "a" $t) (result $t)
      (canon lift (core func $i "f") gc))
  )
  (instance $a (instantiate $A (with "t" (type $e))))
  (alias export $a "f" (func $af))

  (core type $sig (func (param i32) (result i32)))
  (core func $l (canon lower (func $af) gc (core-type $sig)))

  (core module $m
    (import "" "f" (func $f (param i32) (result i32)))
    (func (export "g") (param i32) (result i32) local.get 0 call $f)
  )
  (core instance $i (instantiate $m (with "" (instance (export "f" (func $l))))))

  (export $te "t" (type $e))
  (func (export "g") (param "a" $te) (result $te)
    (canon lift (core func $i "g") gc))
)

(assert_return (invoke "g" (enum.const "red")) (enum.const "red"))
(assert_return (invoke "g" (enum.const "green")) (enum.const "green"))
(assert_return (invoke "g" (enum.const "blue")) (enum.const "blue"))

;; A GC-lowered caller calling a linear-memory-lifted callee.
(component
  (type $e (enum "red" "green" "blue"))
  (component $A
    (import "t" (type $t (eq $e)))
    (core module $m
      (memory (export "m") 1)
      (func (export "f") (param i32) (result i32) local.get 0)
    )
    (core instance $i (instantiate $m))
    (func (export "f") (param "a" $t) (result $t)
      (canon lift (core func $i "f") (memory (core memory $i "m"))))
  )
  (instance $a (instantiate $A (with "t" (type $e))))
  (alias export $a "f" (func $af))

  (core type $sig (func (param i32) (result i32)))
  (core func $l (canon lower (func $af) gc (core-type $sig)))

  (core module $m
    (import "" "f" (func $f (param i32) (result i32)))
    (func (export "g") (param i32) (result i32) local.get 0 call $f)
  )
  (core instance $i (instantiate $m (with "" (instance (export "f" (func $l))))))

  (export $te "t" (type $e))
  (func (export "g") (param "a" $te) (result $te)
    (canon lift (core func $i "g") gc))
)

(assert_return (invoke "g" (enum.const "red")) (enum.const "red"))
(assert_return (invoke "g" (enum.const "green")) (enum.const "green"))
(assert_return (invoke "g" (enum.const "blue")) (enum.const "blue"))

;; A linear-memory-lowered caller calling a GC-lifted callee.
(component
  (type $e (enum "red" "green" "blue"))
  (component $A
    (import "t" (type $t (eq $e)))
    (core module $m
      (func (export "f") (param i32) (result i32) local.get 0)
    )
    (core instance $i (instantiate $m))
    (func (export "f") (param "a" $t) (result $t)
      (canon lift (core func $i "f") gc))
  )
  (instance $a (instantiate $A (with "t" (type $e))))
  (alias export $a "f" (func $af))

  (core func $l (canon lower (func $af)))

  (core module $m
    (import "" "f" (func $f (param i32) (result i32)))
    (memory (export "m") 1)
    (func (export "g") (param i32) (result i32) local.get 0 call $f)
  )
  (core instance $i (instantiate $m (with "" (instance (export "f" (func $l))))))

  (export $te "t" (type $e))
  (func (export "g") (param "a" $te) (result $te)
    (canon lift (core func $i "g") (memory (core memory $i "m"))))
)

(assert_return (invoke "g" (enum.const "red")) (enum.const "red"))
(assert_return (invoke "g" (enum.const "green")) (enum.const "green"))
(assert_return (invoke "g" (enum.const "blue")) (enum.const "blue"))

;; An `enum` discriminant at the case count is out of range.
(component
  (type $e (enum "red" "green" "blue"))
  (component $A
    (import "t" (type $t (eq $e)))
    (core module $m
      (func (export "f") (param i32) (result i32) i32.const 3)
    )
    (core instance $i (instantiate $m))
    (func (export "f") (param "a" $t) (result $t)
      (canon lift (core func $i "f") gc))
  )
  (instance $a (instantiate $A (with "t" (type $e))))
  (alias export $a "f" (func $af))

  (core type $sig (func (param i32) (result i32)))
  (core func $l (canon lower (func $af) gc (core-type $sig)))

  (core module $m
    (import "" "f" (func $f (param i32) (result i32)))
    (func (export "g") (param i32) (result i32) local.get 0 call $f)
  )
  (core instance $i (instantiate $m (with "" (instance (export "f" (func $l))))))

  (export $te "t" (type $e))
  (func (export "g") (param "a" $te) (result $te)
    (canon lift (core func $i "g") gc))
)

(assert_trap (invoke "g" (enum.const "red")) "invalid variant discriminant")

;; A wildly out-of-range `enum` discriminant also traps.
(component
  (type $e (enum "red" "green" "blue"))
  (component $A
    (import "t" (type $t (eq $e)))
    (core module $m
      (func (export "f") (param i32) (result i32) i32.const 0xffffffff)
    )
    (core instance $i (instantiate $m))
    (func (export "f") (param "a" $t) (result $t)
      (canon lift (core func $i "f") gc))
  )
  (instance $a (instantiate $A (with "t" (type $e))))
  (alias export $a "f" (func $af))

  (core type $sig (func (param i32) (result i32)))
  (core func $l (canon lower (func $af) gc (core-type $sig)))

  (core module $m
    (import "" "f" (func $f (param i32) (result i32)))
    (func (export "g") (param i32) (result i32) local.get 0 call $f)
  )
  (core instance $i (instantiate $m (with "" (instance (export "f" (func $l))))))

  (export $te "t" (type $e))
  (func (export "g") (param "a" $te) (result $te)
    (canon lift (core func $i "g") gc))
)

(assert_trap (invoke "g" (enum.const "red")) "invalid variant discriminant")
