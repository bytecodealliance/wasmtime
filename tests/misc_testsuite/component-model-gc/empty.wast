;;! component_model_gc = true
;;! gc = true
;;! reference_types = true
;;! gc_types = true
;;! multi_memory = true

;; GC calling GC.
(component
  (component $A
    (core module $m
      (func (export "f"))
    )
    (core instance $i (instantiate $m))

    (core type $ty (func))
    (func (export "f")
      (canon lift (core func $i "f") gc)
    )
  )

  (component $B
    (import "f" (func $f))

    (core type $ty (func))
    (core func $f' (canon lower (func $f) gc (core-type $ty)))

    (core module $m
      (import "" "f" (func $f))
      (func (export "f")
        call $f
      )
    )

    (core instance $i (instantiate $m
      (with "" (instance (export "f" (func $f'))))
    ))

    (func (export "f") (canon lift (core func $i "f")))
  )

  (instance $a (instantiate $A))
  (instance $b (instantiate $B (with "f" (func $a "f"))))

  (func (export "f") (alias export $b "f"))
)

(assert_return (invoke "f"))

;; GC calling linear memory.
(component
  (component $A
    (core module $m
      (func (export "f"))
    )
    (core instance $i (instantiate $m))

    (core type $ty (func))
    (func (export "f")
      (canon lift (core func $i "f"))
    )
  )

  (component $B
    (import "f" (func $f))

    (core type $ty (func))
    (core func $f' (canon lower (func $f) gc (core-type $ty)))

    (core module $m
      (import "" "f" (func $f))
      (func (export "f")
        call $f
      )
    )

    (core instance $i (instantiate $m
      (with "" (instance (export "f" (func $f'))))
    ))

    (func (export "f") (canon lift (core func $i "f")))
  )

  (instance $a (instantiate $A))
  (instance $b (instantiate $B (with "f" (func $a "f"))))

  (func (export "f") (alias export $b "f"))
)

(assert_return (invoke "f"))

;; Linear memory calling GC.
(component
  (component $A
    (core module $m
      (func (export "f"))
    )
    (core instance $i (instantiate $m))

    (core type $ty (func))
    (func (export "f")
      (canon lift (core func $i "f") gc)
    )
  )

  (component $B
    (import "f" (func $f))

    (core func $f' (canon lower (func $f)))

    (core module $m
      (import "" "f" (func $f))
      (func (export "f")
        call $f
      )
    )

    (core instance $i (instantiate $m
      (with "" (instance (export "f" (func $f'))))
    ))

    (func (export "f") (canon lift (core func $i "f")))
  )

  (instance $a (instantiate $A))
  (instance $b (instantiate $B (with "f" (func $a "f"))))

  (func (export "f") (alias export $b "f"))
)

(assert_return (invoke "f"))
