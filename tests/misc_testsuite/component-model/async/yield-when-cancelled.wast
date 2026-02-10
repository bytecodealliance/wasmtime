;;! component_model_async = true
;;! reference_types = true

;; This test checks that an `EVENT_CANCELLED` can be delivered to a task that's
;; in a `CALLBACK_CODE_YIELD` loop.
(component
  (component $a
    (core module $m
      (import "" "task.cancel" (func $task-cancel))
      (import "" "thread.yield" (func $thread-yield (result i32)))

      (func (export "f") (result i32)
         ;; first, yield in a non-cancellable way a few times to give the caller
         ;; a chance to queue up an `EVENT_CANCELLED`
         (local $i i32)
         (loop $loop
           (i32.ne (i32.const 0 (; NOT_CANCELLED ;) (call $thread-yield)))
           if unreachable end
           (local.set $i (i32.add (i32.const 1) (local.get $i)))
           (i32.ne (i32.const 10) (local.get $i))
           br_if $loop
         )

         i32.const 1 ;; CALLBACK_CODE_YIELD
      )

      (func (export "f-callback") (param i32 i32 i32) (result i32)
         (i32.eq (i32.const 6 (; EVENT_CANCELLED ;)) (local.get 0))
         (if (result i32)
           (then
             call $task-cancel
             i32.const 0 ;; CALLBACK_CODE_EXIT
           )
           (else
             i32.const 1 ;; CALLBACK_CODE_YIELD
           )
         )
      )
    )

    (core func $task-cancel (canon task.cancel))
    (core func $thread-yield (canon thread.yield))

    (core instance $i (instantiate $m
      (with "" (instance
        (export "task.cancel" (func $task-cancel))
        (export "thread.yield" (func $thread-yield))
      ))
    ))

    (func (export "f") async (canon lift (core func $i "f") async (callback (func $i "f-callback"))))
  )
  (instance $a (instantiate $a))

  (component $b
    (import "a" (instance $a
      (export "f" (func async))
    ))

    (core module $m
      (import "" "f" (func $f (result i32)))
      (import "" "subtask.cancel" (func $subtask-cancel (param i32) (result i32)))

      (func (export "f")
         (local $status i32)
         (local $subtask i32)

         (local.set $status (call $f))
         (local.set $subtask (i32.shr_u (local.get $status) (i32.const 4)))
         (local.set $status (i32.and (i32.const 0xF) (local.get $status)))
         (i32.ne (i32.const 1 (; STATUS_STARTED ;)) (local.get $status))
         if unreachable end
         (i32.ne (i32.const 4 (; STATUS_RETURN_CANCELLED ;)) (call $subtask-cancel (local.get $subtask)))
         if unreachable end
      )
    )

    (core func $f (canon lower (func $a "f") async))
    (core func $subtask-cancel (canon subtask.cancel))

    (core instance $i (instantiate $m
      (with "" (instance
        (export "f" (func $f))
        (export "subtask.cancel" (func $subtask-cancel))
      ))
    ))

    (func (export "f") async (canon lift (core func $i "f")))
  )
  (instance $b (instantiate $b
     (with "a" (instance $a))
  ))

  (func (export "f") (alias export $b "f"))
)

(assert_return (invoke "f"))
