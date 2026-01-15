;;! component_model_async = true
;;! reference_types = true
;;! gc_types = true
;;! multi_memory = true

(component
  (component $a
    (core module $a
      (import "" "yield" (func $yield (result i32)))

      (func (export "yield-loop") (result i32)
        ;; simulate `waitable-set.poll` with a yield loop
        (loop
          call $yield
          drop
          br 0
        )
        unreachable
      )

      ;; not reached
      (func (export "callback") (param i32 i32 i32) (result i32) unreachable)

      (func (export "noop"))
    )
    (core func $yield (canon thread.yield))
    (core instance $a (instantiate $a
      (with "" (instance
        (export "yield" (func $yield))
      ))
    ))
    (func (export "yield-loop") async
      (canon lift
        (core func $a "yield-loop")
        async
        (callback (func $a "callback"))
      )
    )
    (func (export "noop") (canon lift (core func $a "noop")))
  )
  (instance $a (instantiate $a))

  (component $b
    (import "yield-loop" (func $yield-loop async))
    (import "noop" (func $noop))

    (core func $yield-loop (canon lower (func $yield-loop) async))
    (core func $noop (canon lower (func $noop)))

    (core module $b
      (import "" "yield-loop" (func $yield-loop (result i32)))
      (import "" "noop" (func $noop))

      (func (export "run")
        ;; call `yield-loop`, double-check it's in the "started" state.
        call $yield-loop
        i32.const 0xf
        i32.and
        i32.const 1
        i32.ne
        if unreachable end

        ;; now try to reenter the other component with some other function.
        call $noop
      )
    )
    (core instance $b (instantiate $b
      (with "" (instance
        (export "yield-loop" (func $yield-loop))
        (export "noop" (func $noop))
      ))
    ))
    (func (export "run") async (canon lift (core func $b "run")))
  )
  (instance $b (instantiate $b
    (with "yield-loop" (func $a "yield-loop"))
    (with "noop" (func $a "noop"))
  ))
  (export "run" (func $b "run"))
)

(assert_return (invoke "run"))
