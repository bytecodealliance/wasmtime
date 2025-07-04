;;! component_model_async = true
;;! reference_types = true
;;! gc_types = true
;;! multi_memory = true

(component
  (component $child
    (core module $libc (memory (export "memory") 1))
    (core instance $libc (instantiate $libc))

    (core module $m
      (import "" "waitable-set-new" (func $waitable-set-new (result i32)))
      (func (export "run") (result i32)
        call $waitable-set-new
        i32.const 4
        i32.shl
        i32.const 2 ;; CallbackCode.WAIT
        i32.or
      )

      (func (export "cb") (param i32 i32 i32) (result i32)
        unreachable)
    )

    (core func $waitable-set-new (canon waitable-set.new))

    (core instance $i (instantiate $m
      (with "" (instance
        (export "waitable-set-new" (func $waitable-set-new))
      ))
    ))

    (func (export "run")
      (canon lift (core func $i "run") async (callback (func $i "cb"))))
  )
  (instance $child (instantiate $child))

  (core func $child-run (canon lower (func $child "run")))

  (core module $m
    (import "" "child-run" (func $child-run))

    (func (export "run")
      (call $child-run))
  )
  (core instance $i (instantiate $m
    (with "" (instance
      (export "child-run" (func $child-run))
    ))
  ))

  (func (export "run")
    (canon lift (core func $i "run")))
)

(assert_trap (invoke "run") "deadlock detected")
