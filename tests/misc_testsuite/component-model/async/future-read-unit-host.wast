;;! component_model_async = true

;; Reading a payload-less future written by the host never uses the pointer
;; passed to `future.read`, so it's valid without a `memory` and with any
;; pointer.
(component
  (import "host" (instance $host
    (export "unit-future" (func (result (future))))
  ))
  (type $future (future))

  (core func $unit-future (canon lower (func $host "unit-future")))
  (core func $future.read (canon future.read $future async))
  (core func $task.return (canon task.return (result u32)))

  (core module $m
    (import "" "unit-future" (func $unit-future (result i32)))
    (import "" "future.read" (func $future.read (param i32 i32) (result i32)))
    (import "" "task.return" (func $task.return (param i32)))
    (func (export "run") (result i32)
      (call $task.return
        (call $future.read (call $unit-future) (i32.const 0x1234)))
      (i32.const 0 (; EXIT ;)))
    (func (export "cb") (param i32 i32 i32) (result i32) unreachable)
  )
  (core instance $i (instantiate $m
    (with "" (instance
      (export "unit-future" (func $unit-future))
      (export "future.read" (func $future.read))
      (export "task.return" (func $task.return))
    ))
  ))

  (func (export "run") async (result u32)
    (canon lift (core func $i "run") async (callback (core func $i "cb"))))
)

;; COMPLETED
(assert_return (invoke "run") (u32.const 0))
