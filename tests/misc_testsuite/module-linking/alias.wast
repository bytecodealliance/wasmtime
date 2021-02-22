;; functions
(module
  (module $m
    (func $foo (export "foo") (result i32)
      i32.const 1)
  )
  (instance $a (instantiate $m))

  (func (export "get") (result i32)
    call (func $a "foo"))
)
(assert_return (invoke "get") (i32.const 1))

;; globals
(module
  (module $m
    (global $g (export "g") (mut i32) (i32.const 2))
  )
  (instance $a (instantiate $m))

  (func (export "get") (result i32)
    global.get (global $a "g"))
)
(assert_return (invoke "get") (i32.const 2))

;; memories
(module
  (module $m
    (memory $m (export "m") 1)
    (data (i32.const 0) "\03\00\00\00")
  )
  (instance $a (instantiate $m))
  (alias $a "m" (memory $m))

  (func (export "get") (result i32)
    i32.const 0
    i32.load)
)
(assert_return (invoke "get") (i32.const 3))

;; tables
(module
  (module $m
    (table $t (export "t") 1 funcref)
    (func (result i32)
      i32.const 4)
    (elem (i32.const 0) 0)
  )
  (instance $a (instantiate $m))

  (func (export "get") (result i32)
    i32.const 0
    call_indirect (table $a "t") (result i32))
)
(assert_return (invoke "get") (i32.const 4))

;; modules
(module
  (module $m
    (module $sub (export "module")
      (func $f (export "") (result i32)
        i32.const 5))
  )
  (instance $a (instantiate $m))
  (instance $b (instantiate (module $a "module")))

  (func (export "get") (result i32)
    call (func $b ""))
)
(assert_return (invoke "get") (i32.const 5))

;; instances
(module
  (module $m
    (module $sub
      (func $f (export "") (result i32)
        i32.const 6))
    (instance $i (export "") (instantiate $sub))
  )
  (instance $a (instantiate $m))

  (func (export "get") (result i32)
    call (func $a "" ""))
)
(assert_return (invoke "get") (i32.const 6))

;; alias parent -- type
(module
  (type $t (func))
  (module $m
    (func $f (type outer 0 $t))
  )
  (instance $a (instantiate $m))
)

;; alias outer -- module
(module
  (module $a)
  (module $m
    (instance (instantiate (module outer 0 $a)))
  )
  (instance (instantiate $m))
)

;; The alias, import, type, module, and instance sections can all be interleaved
(module $ROOT
  (module $a)
  (type $t (func))
  (module $m
    ;; alias
    (alias outer 0 $t (type $thunk))
    ;; import
    (import "" "" (func (type $thunk)))
    ;; module (referencing parent type)
    (module
      (func (type outer $m $thunk))
      (func (type outer $ROOT $t))
    )
    ;; type
    (type $thunk2 (func))
    ;; module (referencing previous alias)
    (module $m2
      (func (export "") (type outer $m $thunk2))
    )
    ;; instance
    (instance $i (instantiate $m2))
    ;; alias that instance
    (alias $i "" (func $my_f))
    ;; module
    (module $m3
      (import "" (func)))
    ;; use our aliased function to create the module
    (instance $i2 (instantiate $m3 (import "" (func $my_f))))
    ;; module
    (module $m4
      (import "" (func)))
  )

  ;; instantiate the above module
  (module $smol (func $f (export "")))
  (instance $smol (instantiate $smol))
  (instance (instantiate $m (import "" (instance $smol))))
)
