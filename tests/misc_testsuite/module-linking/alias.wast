;; functions
(module
  (module $m
    (func $foo (export "foo") (result i32)
      i32.const 1)
  )
  (instance $a (instantiate $m))

  (func (export "get") (result i32)
    call $a.$foo)
)
(assert_return (invoke "get") (i32.const 1))

;; globals
(module
  (module $m
    (global $g (export "g") (mut i32) (i32.const 2))
  )
  (instance $a (instantiate $m))

  (func (export "get") (result i32)
    global.get $a.$g)
)
(assert_return (invoke "get") (i32.const 2))

;; memories
(module
  (module $m
    (memory $m (export "m") 1)
    (data (i32.const 0) "\03\00\00\00")
  )
  (instance $a (instantiate $m))
  (alias (instance $a) (memory $m))

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
    call_indirect $a.$t (result i32))
)
(assert_return (invoke "get") (i32.const 4))

;; TODO instances/modules -- needs import/export of modules/instances to work

;; alias parent -- type
(module
  (type $t (func))
  (module $m
    (func $f (type $t))
  )
  (instance $a (instantiate $m))
)

;; alias parent -- module
(module
  (module $a)
  (module $m
    (instance (instantiate $a))
  )
  (instance (instantiate $m))
)

;; The alias, import, type, module, and instance sections can all be interleaved
(module
  (module $a)
  (type $t (func))
  (module $m
    ;; alias
    (alias $thunk parent (type $t))
    ;; import
    (import "" "" (func (type $thunk)))
    ;; module (referencing parent type)
    (module
      (func (type $thunk))
    )
    ;; type
    (type $thunk2 (func))
    ;; module (referencing previous alias)
    (module $m2
      (func (export "") (type $thunk2))
    )
    ;; instance
    (instance $i (instantiate $m2))
    ;; alias that instance
    (alias $my_f (instance $i) (func 0))
    ;; module
    (module $m3
      (import "" (func)))
    ;; use our aliased function to create the module
    (instance $i2 (instantiate $m3 (func $my_f)))
    ;; module
    (module $m4
      (import "" (func)))
  )

  ;; instantiate the above module
  (module $smol (func $f (export "")))
  (instance $smol (instantiate $smol))
  (instance (instantiate $m (func $smol.$f)))
)
