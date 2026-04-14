;;! component_model_async = true
;;! reference_types = true

;; This exposes a historical bug in wasmtime where when a guest subtask was
;; dropped in the `STARTING` state it leaked resources within the store. Here
;; this is done in a loop N times after setting the store's table capacity much
;; lower than the loop iterations.

(component
  (import "wasmtime" (instance $wasmtime
    (export "set-max-table-capacity" (func (param "max" u32)))
  ))

  (component $A
    (core module $m
      (import "" "backpressure.inc" (func $backpressure.inc))

      (func (export "set-backpressure") (call $backpressure.inc))
      (func (export "hi"))
    )
    (core func $backpressure.inc (canon backpressure.inc))
    (core instance $i (instantiate $m
      (with "" (instance
        (export "backpressure.inc" (func $backpressure.inc))
      ))
    ))

    (func (export "set-backpressure") (canon lift (core func $i "set-backpressure")))
    (func (export "hi") async (canon lift (core func $i "hi")))
  )
  (instance $a (instantiate $A))

  (component $B
    (import "wasmtime" (instance $wasmtime
      (export "set-max-table-capacity" (func (param "max" u32)))
    ))
    (import "a" (instance $a
      (export "set-backpressure" (func))
      (export "hi" (func async))
    ))

    (core func $set-backpressure (canon lower (func $a "set-backpressure")))
    (core func $hi (canon lower (func $a "hi") async))
    (core func $set-max-table-capacity (canon lower (func $wasmtime "set-max-table-capacity")))
    (core func $subtask.cancel (canon subtask.cancel))
    (core func $subtask.drop (canon subtask.drop))

    (core module $m
      (import "" "set-backpressure" (func $set-backpressure))
      (import "" "hi" (func $hi (result i32)))
      (import "" "subtask.cancel" (func $subtask.cancel (param i32) (result i32)))
      (import "" "subtask.drop" (func $subtask.drop (param i32)))
      (import "" "set-max-table-capacity" (func $set-max-table-capacity (param i32)))

      (func (export "run")
        (local $rc i32)
        (local $task i32)
        (local $cnt i32)
        call $set-backpressure

        (call $set-max-table-capacity (i32.const 100))

        (local.set $cnt (i32.const 1000))

        loop $l
          (local.set $rc (call $hi))
          (if (i32.ne (i32.and (local.get $rc) (i32.const 0xf)) (i32.const 0 (; STARTING ;)))
            (then unreachable))
          (local.set $task (i32.shr_u (local.get $rc) (i32.const 4)))
          (local.set $rc (call $subtask.cancel (local.get $task)))
          (if (i32.ne (i32.and (local.get $rc) (i32.const 0xf)) (i32.const 3 (; START_CANCELLED ;)))
            (then unreachable))

          (call $subtask.drop (local.get $task))

          (local.set $cnt (i32.sub (local.get $cnt) (i32.const 1)))
          (if (local.get $cnt)
            (then (br $l)))
        end
      )
    )

    (core instance $i (instantiate $m
      (with "" (instance
        (export "set-backpressure" (func $set-backpressure))
        (export "hi" (func $hi))
        (export "subtask.cancel" (func $subtask.cancel))
        (export "subtask.drop" (func $subtask.drop))
        (export "set-max-table-capacity" (func $set-max-table-capacity))
      ))
    ))

    (func (export "run") async (canon lift (core func $i "run")))
  )

  (instance $b (instantiate $B
    (with "a" (instance $a))
    (with "wasmtime" (instance $wasmtime))
  ))
  (export "run" (func $b "run"))
)

(assert_return (invoke "run"))
