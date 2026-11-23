;;! component_model_async = true

;; This tests that, when a call is queued behind an earlier call (which itself
;; was queued due to backpressure which is no longer present, but the task has
;; not yet had a chance to start), and then the second call is cancelled before
;; it can start, the first call should proceed.
(component
  (component $C
    (core func $backpressure.inc (canon backpressure.inc))
    (core func $backpressure.dec (canon backpressure.dec))
    (func (export "bp-on") (canon lift (core func $backpressure.inc)))
    (func (export "bp-off") (canon lift (core func $backpressure.dec)))

    (core func $task.return (canon task.return))
    (core module $m
      (import "" "task.return" (func $task.return))
      (func (export "f") (result i32)
        (call $task.return)
        (i32.const 0)) ;; CALLBACK_CODE_EXIT
      (func (export "f-cb") (param i32 i32 i32) (result i32) unreachable)
    )
    (core instance $i (instantiate $m
      (with "" (instance
        (export "task.return" (func $task.return))
      ))
    ))
    (func (export "f") async
      (canon lift (core func $i "f") async (callback (core func $i "f-cb"))))
  )
  (instance $c (instantiate $C))

  (component $A
    (import "c" (instance $c
      (export "bp-on" (func))
      (export "bp-off" (func))
      (export "f" (func async))
    ))
    (core module $libc (memory (export "mem") 1))
    (core instance $libc (instantiate $libc))
    (core func $bp-on (canon lower (func $c "bp-on")))
    (core func $bp-off (canon lower (func $c "bp-off")))
    (core func $f (canon lower (func $c "f") async (memory (core memory $libc "mem"))))
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
        ;; low 4 bits are the subtask status; 0 = STARTING
        (if (i32.ne (i32.and (local.get $st) (i32.const 0xf)) (i32.const 0))
          (then unreachable))
        ;; the subtask handle is in the bits above the 4-bit status
        (i32.shr_u (local.get $st) (i32.const 4)))

      ;; Cancels the not-yet-started subtask `$h` and drops it.
      (func $cancel (param $h i32)
        ;; 3 = CANCELLED_BEFORE_STARTED
        (if (i32.ne (call $subtask.cancel (local.get $h)) (i32.const 3))
          (then unreachable))
        (call $subtask.drop (local.get $h)))

      (func (export "run") (result i32)
        (local $h1 i32)
        (global.set $set (call $waitable-set.new))
        (call $bp-on)
        ;; first call is queued behind backpressure => STARTING
        (local.set $h1 (call $start-f))
        (call $bp-off)
        ;; second call: queued behind h1 => STARTING
        (call $waitable.join (call $start-f) (global.get $set))
        ;; cancel the call ahead of it; the second call must still run
        (call $cancel (local.get $h1))
        (global.set $pending (i32.const 1))
        ;; CALLBACK_CODE_WAIT (2) on `$set`, which goes in the bits above 4
        (i32.or (i32.shl (global.get $set) (i32.const 4)) (i32.const 2)))

      (func (export "cb") (param $ev i32) (param $h i32) (param $status i32) (result i32)
        ;; 1 = EVENT_SUBTASK
        (if (i32.ne (local.get $ev) (i32.const 1)) (then unreachable))
        ;; 2 = RETURNED
        (if (i32.eq (local.get $status) (i32.const 2))
          (then
            (call $subtask.drop (local.get $h))
            (global.set $pending (i32.sub (global.get $pending) (i32.const 1)))))
        (if (i32.eqz (global.get $pending))
          (then
            (call $task.return)
            (return (i32.const 0)))) ;; CALLBACK_CODE_EXIT
        ;; CALLBACK_CODE_WAIT (2) on `$set`, which goes in the bits above 4
        (i32.or (i32.shl (global.get $set) (i32.const 4)) (i32.const 2)))
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
  )
  (instance $a (instantiate $A (with "c" (instance $c))))
  (export "run" (func $a "run"))
)

(assert_return (invoke "run"))
