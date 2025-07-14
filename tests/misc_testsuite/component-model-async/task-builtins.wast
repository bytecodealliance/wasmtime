;;! component_model_async = true
;;! component_model_async_stackful = true

;; backpressure.set
(component
  (core module $m
    (import "" "backpressure.set" (func $backpressure-set (param i32)))
  )
  (core func $backpressure-set (canon backpressure.set))
  (core instance $i (instantiate $m (with "" (instance (export "backpressure.set" (func $backpressure-set))))))
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
  (core func $waitable-set-wait (canon waitable-set.wait async (memory $libc "memory")))
  (core instance $i (instantiate $m (with "" (instance (export "waitable-set.wait" (func $waitable-set-wait))))))
)

;; waitable-set.poll
(component
  (core module $libc (memory (export "memory") 1))
  (core instance $libc (instantiate $libc))
  (core module $m
    (import "" "waitable-set.poll" (func $waitable-set-poll (param i32 i32) (result i32)))
  )
  (core func $waitable-set-poll (canon waitable-set.poll async (memory $libc "memory")))
  (core instance $i (instantiate $m (with "" (instance (export "waitable-set.poll" (func $waitable-set-poll))))))
)

;; yield
(component
  (core module $m
    (import "" "yield" (func $yield (result i32)))
  )
  (core func $yield (canon yield async))
  (core instance $i (instantiate $m (with "" (instance (export "yield" (func $yield))))))
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
