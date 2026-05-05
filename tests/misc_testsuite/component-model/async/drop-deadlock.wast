;;! component_model_async = true

(component
  (core func $backpressure_inc (canon backpressure.inc))

  (core module $m
    (import "" "backpressure.inc" (func $backpressure_inc))
    (func (export "set-backpressure") (call $backpressure_inc))
    (func (export "target") (result i32) unreachable)
    (func (export "callback") (param i32 i32 i32) (result i32) unreachable)
  )

  (core instance $i (instantiate $m
    (with "" (instance (export "backpressure.inc" (func $backpressure_inc))))
  ))

  (func (export "set-backpressure")
    (canon lift (core func $i "set-backpressure")))

  (func (export "target")
    (canon lift (core func $i "target") async (callback (func $i "callback"))))
)

(assert_return (invoke "set-backpressure"))
(assert_trap (invoke "target") "cannot make further progress")
