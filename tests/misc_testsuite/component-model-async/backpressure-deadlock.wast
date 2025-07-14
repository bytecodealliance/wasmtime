;;! component_model_async = true
;;! reference_types = true
;;! gc_types = true
;;! multi_memory = true

;; - Component B asks component A to enable backpressure
;; - Component B makes an async call to component A
;; - Component B asserts this subtask is in the "STARTING" state
;; - Component B adds the subtask to a waitable set and calls waitable-set.wait
;;
;; This leaves both tasks in a deadlock situation, which, as of this writing,
;; Wasmtime will handle by trapping.  In the future, once there's a host API for
;; cancelling tasks, that behavior may change, in which case this test will need
;; to be updated.
(component

  (component $A
    (core func $backpressure.set (canon backpressure.set))
    (core module $m
      (import "" "backpressure.set" (func $backpressure.set (param i32)))

      (func (export "f") (result i32) unreachable)
      (func (export "callback") (param i32 i32 i32) (result i32) unreachable)

      (func (export "turn-on-backpressure")
        (call $backpressure.set (i32.const 1)))
    )

    (core instance $i (instantiate $m
      (with "" (instance
        (export "backpressure.set" (func $backpressure.set))
      ))
    ))

    (func (export "turn-on-backpressure") (canon lift (core func $i "turn-on-backpressure")))
    (func (export "f")
      (canon lift (core func $i "f") async (callback (func $i "callback"))))
  )
  (instance $A (instantiate $A))

  (core module $libc (memory (export "mem") 1))
  (core instance $libc (instantiate $libc))

  (core func $f (canon lower (func $A "f") async (memory $libc "mem")))
  (core func $turn-on-backpressure (canon lower (func $A "turn-on-backpressure")))
  (core func $waitable-set.new (canon waitable-set.new))
  (core func $waitable.join (canon waitable.join))
  (core func $waitable-set.wait (canon waitable-set.wait (memory $libc "mem")))

  (core module $m
    (import "" "f" (func $f (result i32)))
    (import "" "turn-on-backpressure" (func $turn-on-backpressure))
    (import "" "waitable-set.new" (func $waitable-set.new (result i32)))
    (import "" "waitable.join" (func $waitable.join (param i32 i32)))
    (import "" "waitable-set.wait" (func $waitable-set.wait (param i32 i32) (result i32)))

    (func (export "f")
      (local $status i32)
      (local $set i32)
      call $turn-on-backpressure

      (local.set $status (call $f))

      ;; low 4 bits should be "STARTING == 0"
      (i32.ne
        (i32.const 0)
        (i32.and
          (local.get $status)
          (i32.const 0xf)))
      if unreachable end

      ;; make a new waitable set and join our subtask into it
      (local.set $set (call $waitable-set.new))
      (call $waitable.join
        (i32.shr_u (local.get $status) (i32.const 4))
        (local.get $set))

      ;; block waiting for our task, which should deadlock (?)
      (call $waitable-set.wait (local.get $set) (i32.const 0))
      unreachable
    )
  )

  (core instance $i (instantiate $m
    (with "" (instance
      (export "f" (func $f))
      (export "turn-on-backpressure" (func $turn-on-backpressure))
      (export "waitable-set.new" (func $waitable-set.new))
      (export "waitable.join" (func $waitable.join))
      (export "waitable-set.wait" (func $waitable-set.wait))
    ))
  ))

  (func (export "f") (canon lift (core func $i "f")))
)

(assert_trap (invoke "f") "deadlock detected")
