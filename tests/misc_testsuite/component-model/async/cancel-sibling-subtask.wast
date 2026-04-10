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
      (import "" "future.new" (func $future.new (result i64)))
      (global $subtask (mut i32) (i32.const 0))

      ;; This export starts a call to `$f` in the above component but doesn't
      ;; await it or complete it. Instead this task exits.
      (func (export "a") (param $cancel i32)
        (local $ret i32)

        (drop (call $future.new))

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
    (type $ft (future))
    (core func $future.new (canon future.new $ft))
    (core instance $i (instantiate $m
      (with "" (instance
        (export "f" (func $f))
        (export "cancel" (func $cancel))
        (export "drop" (func $drop))
        (export "future.new" (func $future.new))
      ))
    ))
    (func (export "a") async (param "cancel" bool) (canon lift (core func $i "a")))
    (func (export "b") async (param "cancel" bool) (canon lift (core func $i "b")))
  )

  (component $c
    (import "b" (instance $b
      (export "a" (func async (param "cancel" bool)))
      (export "b" (func async (param "cancel" bool)))
    ))
    (core module $m
      (import "" "a" (func $a (param i32)))
      (import "" "b" (func $b (param i32)))
      (import "" "future.new" (func $future.new (result i64)))

      (func (export "run") (param i32 i32)
        (call $a (local.get 0))

        ;; push things into the handle table to ensure that b's task is
        ;; different from a's.
        (drop (call $future.new))

        (call $b (local.get 1))
      )
    )
    (core func $a (canon lower (func $b "a")))
    (core func $b (canon lower (func $b "b")))
    (type $ft (future))
    (core func $future.new (canon future.new $ft))
    (core instance $i (instantiate $m
      (with "" (instance
        (export "a" (func $a))
        (export "b" (func $b))
        (export "future.new" (func $future.new))
      ))
    ))

    (func (export "run") async (param "a" bool) (param "b" bool)
      (canon lift (core func $i "run"))
    )
  )

  (instance $a (instantiate $a))
  (instance $b (instantiate $b (with "f" (func $a "f"))))
  (instance $c (instantiate $c (with "b" (instance $b))))
  (export "run" (func $c "run"))
)

;; start subtask in "a", cancel/drop it in "b"
(component instance $A $A)
(assert_return (invoke "run" (bool.const false) (bool.const true)))

;; start/cancel subtask in "a", drop it in "b"
(component instance $A $A)
(assert_return (invoke "run" (bool.const true) (bool.const false)))
