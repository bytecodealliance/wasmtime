;;! reference_types = true
;;! component_model_async = true

(component definition $A
  ;; A component with a single export that's just an infinitely looping
  ;; subtask. Used to represent a pending subtask that has yet to resolve.
  (component $a
    (core module $m
      (import "" "cancel" (func $cancel))
      (func (export "a") (result i32) i32.const 1) ;; CALLBACK_CODE_YIELD
      (func (export "cb") (param i32 i32 i32) (result i32)
        local.get 0
        i32.const 6 ;; EVENT_TASK_CANCELLED
        i32.eq
        if (result i32)
          call $cancel
          i32.const 0 ;; CALLBACK_CODE_EXIT
        else
          i32.const 1 ;; CALLBACK_CODE_YIELD
        end

        )
    )
    (core func $cancel (canon task.cancel))
    (core instance $i (instantiate $m
      (with "" (instance
        (export "cancel" (func $cancel))
      ))
    ))
    (func (export "f") async
      (canon lift (core func $i "a") async (callback (func $i "cb"))))
  )

  (component $b
    (import "f" (func $f async))
    (core module $m
      (import "" "f" (func $f (result i32)))
      (import "" "cancel" (func $cancel (param i32) (result i32)))
      (import "" "drop" (func $drop (param i32)))
      (global $subtask (mut i32) (i32.const 0))

      ;; This export starts a call to `$f` in the above component but doesn't
      ;; await it or complete it. Instead this task exits.
      (func (export "a") (param $cancel i32)
        (local $ret i32)

        ;; start the subtask
        (local.set $ret (call $f))

        ;; verify it's in the `SUBTASK_STARTED` state
        (i32.ne
          (i32.and (local.get $ret) (i32.const 0xf))
          (i32.const 1)) ;; SUBTASK_STARTED
        if unreachable end

        ;; store the subtask id in a global.
        (global.set $subtask
          (i32.shr_u
            (local.get $ret)
            (i32.const 4)))

        local.get $cancel
        if call $call-cancel end
      )

      (func $call-cancel
        (i32.ne
          (call $cancel (global.get $subtask))
          (i32.const 4)) ;; RETURN_CANCELLED
        if unreachable end
      )

      ;; This export tries to cancel/drop the subtask started by "a" above and
      ;; this shouldn't cause any issues...
      (func (export "b") (param $cancel i32)
        local.get $cancel
        if call $call-cancel end
        (call $drop (global.get $subtask))
      )
    )
    (core func $f (canon lower (func $f) async))
    (core func $cancel (canon subtask.cancel))
    (core func $drop (canon subtask.drop))
    (core instance $i (instantiate $m
      (with "" (instance
        (export "f" (func $f))
        (export "cancel" (func $cancel))
        (export "drop" (func $drop))
      ))
    ))
    (func (export "a") async (param "cancel" bool) (canon lift (core func $i "a")))
    (func (export "b") async (param "cancel" bool) (canon lift (core func $i "b")))
  )

  (instance $a (instantiate $a))
  (instance $b (instantiate $b (with "f" (func $a "f"))))
  (export "a" (func $b "a"))
  (export "b" (func $b "b"))
)

;; start subtask in "a", cancel/drop it in "b"
(component instance $A $A)
(assert_return (invoke "a" (bool.const false)))
(assert_return (invoke "b" (bool.const true)))

;; start/cancel subtask in "a", drop it in "b"
(component instance $A $A)
(assert_return (invoke "a" (bool.const true)))
(assert_return (invoke "b" (bool.const false)))
