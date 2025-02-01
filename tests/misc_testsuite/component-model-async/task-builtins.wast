;;! component_model_async = true

;; task.backpressure
(component
  (core module $m
    (import "" "task.backpressure" (func $task-backpressure (param i32)))
  )
  (core func $task-backpressure (canon task.backpressure))
  (core instance $i (instantiate $m (with "" (instance (export "task.backpressure" (func $task-backpressure))))))
)

;; task.return
(component
  (core module $m
    (import "" "task.return" (func $task-return (param i32)))
  )
  (core func $task-return (canon task.return (result u32)))
  (core instance $i (instantiate $m (with "" (instance (export "task.return" (func $task-return))))))
)

;; task.wait
(component
  (core module $libc (memory (export "memory") 1))
  (core instance $libc (instantiate $libc))
  (core module $m
    (import "" "task.wait" (func $task-wait (param i32) (result i32)))
  )
  (core func $task-wait (canon task.wait async (memory $libc "memory")))
  (core instance $i (instantiate $m (with "" (instance (export "task.wait" (func $task-wait))))))
)

;; task.poll
(component
  (core module $libc (memory (export "memory") 1))
  (core instance $libc (instantiate $libc))
  (core module $m
    (import "" "task.poll" (func $task-poll (param i32) (result i32)))
  )
  (core func $task-poll (canon task.poll async (memory $libc "memory")))
  (core instance $i (instantiate $m (with "" (instance (export "task.poll" (func $task-poll))))))
)

;; task.yield
(component
  (core module $m
    (import "" "task.yield" (func $task-yield))
  )
  (core func $task-yield (canon task.yield async))
  (core instance $i (instantiate $m (with "" (instance (export "task.yield" (func $task-yield))))))
)

;; subtask.drop
(component
  (core module $m
    (import "" "subtask.drop" (func $subtask-drop (param i32)))
  )
  (core func $subtask-drop (canon subtask.drop))
  (core instance $i (instantiate $m (with "" (instance (export "subtask.drop" (func $subtask-drop))))))
)
