;;! component_model_async = true

;; `$B` is called asynchronously by `$C` and, before it ever suspends, cancels
;; one of its own subtasks that is queued behind backpressure in `$A`, then
;; makes another async call. The cancel returns `CANCELLED_BEFORE_STARTED`, the
;; next call is queued (`STARTING`), and everything returns normally.
(component
  (component $A
    (core func $backpressure.inc (canon backpressure.inc))
    (core func $task.return (canon task.return))
    (core module $m
      (import "" "backpressure.inc" (func $backpressure.inc))
      (import "" "task.return" (func $task.return))
      (func (export "f") (result i32) (call $backpressure.inc) (call $task.return) i32.const 0)
      (func (export "cb") (param i32 i32 i32) (result i32) unreachable))
    (core instance $i (instantiate $m (with "" (instance
      (export "backpressure.inc" (func $backpressure.inc))
      (export "task.return" (func $task.return))))))
    (func (export "f") async (canon lift (core func $i "f") async (callback (core func $i "cb")))))

  (component $B
    (import "f" (func $f async))
    (core module $libc (memory (export "mem") 1))
    (core instance $libc (instantiate $libc))
    (core func $f (canon lower (func $f) async (memory (core memory $libc "mem"))))
    (core func $subtask.cancel (canon subtask.cancel))
    (core func $task.return (canon task.return))
    (core module $m
      (import "" "f" (func $f (result i32)))
      (import "" "subtask.cancel" (func $subtask.cancel (param i32) (result i32)))
      (import "" "task.return" (func $task.return))
      (func (export "run") (result i32)
        ;; returns eagerly (RETURNED = 2), leaving backpressure enabled in $A
        (if (i32.ne (call $f) (i32.const 2)) (then unreachable))
        ;; queued behind backpressure: subtask 1, STARTING = 0
        (if (i32.ne (call $f) (i32.const 0x10)) (then unreachable))
        ;; CANCELLED_BEFORE_STARTED = 3
        (if (i32.ne (call $subtask.cancel (i32.const 1)) (i32.const 3)) (then unreachable))
        ;; queued again: subtask 2, STARTING
        (if (i32.ne (call $f) (i32.const 0x20)) (then unreachable))
        (call $task.return)
        i32.const 0)
      (func (export "cb") (param i32 i32 i32) (result i32) unreachable))
    (core instance $i (instantiate $m (with "" (instance
      (export "f" (func $f))
      (export "subtask.cancel" (func $subtask.cancel))
      (export "task.return" (func $task.return))))))
    (func (export "run") async (canon lift (core func $i "run") async (callback (core func $i "cb")))))

  (component $C
    (import "run" (func $run async))
    (core module $libc (memory (export "mem") 1))
    (core instance $libc (instantiate $libc))
    (core func $run (canon lower (func $run) async (memory (core memory $libc "mem"))))
    (core func $task.return (canon task.return))
    (core module $m
      (import "" "run" (func $run (result i32)))
      (import "" "task.return" (func $task.return))
      (func (export "run") (result i32)
        ;; $B returns without blocking
        (if (i32.ne (call $run) (i32.const 2)) (then unreachable))
        (call $task.return)
        i32.const 0)
      (func (export "cb") (param i32 i32 i32) (result i32) unreachable))
    (core instance $i (instantiate $m (with "" (instance
      (export "run" (func $run))
      (export "task.return" (func $task.return))))))
    (func (export "run") async (canon lift (core func $i "run") async (callback (core func $i "cb")))))

  (instance $a (instantiate $A))
  (instance $b (instantiate $B (with "f" (func $a "f"))))
  (instance $c (instantiate $C (with "run" (func $b "run"))))
  (export "run" (func $c "run")))

(assert_return (invoke "run"))
