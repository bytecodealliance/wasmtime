;;! component_model_async = true

;; Per the spec's `Task.enter_implicit_thread`, an async-typed call must wait to
;; enter its instance not only while the instance has backpressure enabled but
;; also while other calls are already waiting to enter it
;; (`num_waiting_to_enter > 0`), so a new call can't barge in ahead of calls
;; which were queued behind backpressure that has since been released.
(component
  (component $C
    (core func $backpressure.inc (canon backpressure.inc))
    (core func $backpressure.dec (canon backpressure.dec))
    (core func $task.return (canon task.return))
    (core module $m
      (import "" "backpressure.inc" (func $backpressure.inc))
      (import "" "backpressure.dec" (func $backpressure.dec))
      (import "" "task.return" (func $task.return))
      (func (export "bp-on") (call $backpressure.inc))
      (func (export "bp-off") (call $backpressure.dec))
      (func (export "f") (result i32)
        (call $task.return)
        (i32.const 0))
      (func (export "f-cb") (param i32 i32 i32) (result i32) unreachable)
    )
    (core instance $i (instantiate $m
      (with "" (instance
        (export "backpressure.inc" (func $backpressure.inc))
        (export "backpressure.dec" (func $backpressure.dec))
        (export "task.return" (func $task.return))
      ))
    ))
    (func (export "bp-on") (canon lift (core func $i "bp-on")))
    (func (export "bp-off") (canon lift (core func $i "bp-off")))
    (func (export "f") async
      (canon lift (core func $i "f") async (callback (core func $i "f-cb"))))
  )
  (instance $c (instantiate $C))

  (component $A
    (import "bp-on" (func $bp-on))
    (import "bp-off" (func $bp-off))
    (import "f" (func $f async))
    (core module $libc (memory (export "mem") 1))
    (core instance $libc (instantiate $libc))
    (core func $bp-on (canon lower (func $bp-on)))
    (core func $bp-off (canon lower (func $bp-off)))
    (core func $f (canon lower (func $f) async (memory (core memory $libc "mem"))))
    (core func $subtask.cancel (canon subtask.cancel))
    (core func $subtask.drop (canon subtask.drop))
    (core func $waitable-set.new (canon waitable-set.new))
    (core func $waitable.join (canon waitable.join))
    (core func $task.return (canon task.return))
    (core module $m
      (import "" "bp-on" (func $bp-on))
      (import "" "bp-off" (func $bp-off))
      (import "" "f" (func $f (result i32)))
      (import "" "subtask.cancel" (func $subtask.cancel (param i32) (result i32)))
      (import "" "subtask.drop" (func $subtask.drop (param i32)))
      (import "" "waitable-set.new" (func $waitable-set.new (result i32)))
      (import "" "waitable.join" (func $waitable.join (param i32 i32)))
      (import "" "task.return" (func $task.return))
      (global $set (mut i32) (i32.const 0))
      (global $pending (mut i32) (i32.const 0))

      ;; Calls `f`, asserts that the subtask is STARTING, and returns its
      ;; handle.
      (func $start-f (result i32)
        (local $st i32)
        (local.set $st (call $f))
        (if (i32.ne (i32.and (local.get $st) (i32.const 0xf)) (i32.const 0))
          (then unreachable))
        (i32.shr_u (local.get $st) (i32.const 4)))

      ;; Cancels the not-yet-started subtask `$h` and drops it.
      (func $cancel (param $h i32)
        (if (i32.ne (call $subtask.cancel (local.get $h)) (i32.const 3))
          (then unreachable))
        (call $subtask.drop (local.get $h)))

      (func (export "run") (result i32)
        (global.set $set (call $waitable-set.new))
        (call $bp-on)
        ;; first call is queued behind backpressure => STARTING
        (call $waitable.join (call $start-f) (global.get $set))
        (global.set $pending (i32.const 1))
        (call $bp-off)
        ;; second call: a call is still waiting to enter => STARTING
        (call $waitable.join (call $start-f) (global.get $set))
        (global.set $pending (i32.const 2))
        (i32.or (i32.shl (global.get $set) (i32.const 4)) (i32.const 2)))

      (func (export "cb") (param $ev i32) (param $h i32) (param $status i32) (result i32)
        (if (i32.ne (local.get $ev) (i32.const 1)) (then unreachable))
        (if (i32.eq (local.get $status) (i32.const 2))
          (then
            (call $subtask.drop (local.get $h))
            (global.set $pending (i32.sub (global.get $pending) (i32.const 1)))))
        (if (i32.eqz (global.get $pending))
          (then
            (call $task.return)
            (return (i32.const 0))))
        (i32.or (i32.shl (global.get $set) (i32.const 4)) (i32.const 2)))

      ;; Cancelling calls which are waiting to enter, whether still blocked by
      ;; backpressure or already queued to run after it's released, means
      ;; they're no longer waiting, so a new call can enter immediately.
      (func (export "run-cancel") (result i32)
        (local $h1 i32)
        (local $h2 i32)
        (call $bp-on)
        (local.set $h1 (call $start-f))
        (local.set $h2 (call $start-f))
        (call $cancel (local.get $h1))
        (call $bp-off)
        (call $cancel (local.get $h2))
        ;; nothing is waiting to enter => RETURNED
        (if (i32.ne (call $f) (i32.const 2))
          (then unreachable))
        (call $task.return)
        (i32.const 0))
    )
    (core instance $i (instantiate $m
      (with "" (instance
        (export "bp-on" (func $bp-on))
        (export "bp-off" (func $bp-off))
        (export "f" (func $f))
        (export "subtask.cancel" (func $subtask.cancel))
        (export "subtask.drop" (func $subtask.drop))
        (export "waitable-set.new" (func $waitable-set.new))
        (export "waitable.join" (func $waitable.join))
        (export "task.return" (func $task.return))
      ))
    ))
    (func (export "run") async
      (canon lift (core func $i "run") async (callback (core func $i "cb"))))
    (func (export "run-cancel") async
      (canon lift (core func $i "run-cancel") async (callback (core func $i "cb"))))
  )
  (instance $a (instantiate $A
    (with "bp-on" (func $c "bp-on"))
    (with "bp-off" (func $c "bp-off"))
    (with "f" (func $c "f"))))
  (export "run" (func $a "run"))
  (export "run-cancel" (func $a "run-cancel"))
)

(assert_return (invoke "run"))
(assert_return (invoke "run-cancel"))
