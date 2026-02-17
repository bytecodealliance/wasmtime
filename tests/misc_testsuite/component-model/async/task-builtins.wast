;;! component_model_async = true

;; backpressure.inc
(component
  (core module $m
    (import "" "backpressure.inc" (func $backpressure-inc))
  )
  (core func $backpressure-inc (canon backpressure.inc))
  (core instance $i (instantiate $m (with "" (instance (export "backpressure.inc" (func $backpressure-inc))))))
)

;; backpressure.dec
(component
  (core module $m
    (import "" "backpressure.dec" (func $backpressure-dec))
  )
  (core func $backpressure-dec (canon backpressure.dec))
  (core instance $i (instantiate $m (with "" (instance (export "backpressure.dec" (func $backpressure-dec))))))
)

;; task.return
(component
  (core module $m
    (import "" "task.return" (func $task-return (param i32)))
  )
  (core func $task-return (canon task.return (result u32)))
  (core instance $i (instantiate $m (with "" (instance (export "task.return" (func $task-return))))))
)

;; waitable-set.wait
(component
  (core module $libc (memory (export "memory") 1))
  (core instance $libc (instantiate $libc))
  (core module $m
    (import "" "waitable-set.wait" (func $waitable-set-wait (param i32 i32) (result i32)))
  )
  (core func $waitable-set-wait (canon waitable-set.wait (memory $libc "memory")))
  (core instance $i (instantiate $m (with "" (instance (export "waitable-set.wait" (func $waitable-set-wait))))))
)

;; waitable-set.poll
(component
  (core module $libc (memory (export "memory") 1))
  (core instance $libc (instantiate $libc))
  (core module $m
    (import "" "waitable-set.poll" (func $waitable-set-poll (param i32 i32) (result i32)))
  )
  (core func $waitable-set-poll (canon waitable-set.poll (memory $libc "memory")))
  (core instance $i (instantiate $m (with "" (instance (export "waitable-set.poll" (func $waitable-set-poll))))))
)

;; thread.yield
(component
  (core module $m
    (import "" "thread.yield" (func $thread-yield (result i32)))
  )
  (core func $thread-yield (canon thread.yield))
  (core instance $i (instantiate $m (with "" (instance (export "thread.yield" (func $thread-yield))))))
)

;; subtask.drop
(component
  (core module $m
    (import "" "subtask.drop" (func $subtask-drop (param i32)))
  )
  (core func $subtask-drop (canon subtask.drop))
  (core instance $i (instantiate $m (with "" (instance (export "subtask.drop" (func $subtask-drop))))))
)

;; subtask.cancel
(component
  (core module $m
    (import "" "subtask.cancel" (func $subtask-drop (param i32) (result i32)))
  )
  (core func $subtask-cancel (canon subtask.cancel))
  (core instance $i (instantiate $m (with "" (instance (export "subtask.cancel" (func $subtask-cancel))))))
)

;; Test that some intrinsics are exempt from may-leave checks
(component
  (core func $backpressure.inc (canon backpressure.inc))
  (core func $backpressure.dec (canon backpressure.dec))
  (core func $context.get (canon context.get i32 0))
  (core func $context.set (canon context.set i32 0))

  (core module $DM
    (import "" "backpressure.inc" (func $backpressure.inc))
    (import "" "backpressure.dec" (func $backpressure.dec))
    (import "" "context.get" (func $context.get (result i32)))
    (import "" "context.set" (func $context.set (param i32)))

    (global $g (mut i32) (i32.const 0))

    (func (export "run")
      (call $context.set (i32.const 100))
    )
    (func (export "post-return")
      call $backpressure.inc
      call $backpressure.dec

      ;; context.get should be what was set in `run`
      call $context.get
      i32.const 100
      i32.ne
      if unreachable end

      i32.const 32
      call $context.set
    )
  )
  (core instance $dm (instantiate $DM (with "" (instance
    (export "backpressure.inc" (func $backpressure.inc))
    (export "backpressure.dec" (func $backpressure.dec))
    (export "context.get" (func $context.get))
    (export "context.set" (func $context.set))
  ))))
  (func (export "run")
    (canon lift (core func $dm "run") (post-return (func $dm "post-return"))))
)

(assert_return (invoke "run"))
