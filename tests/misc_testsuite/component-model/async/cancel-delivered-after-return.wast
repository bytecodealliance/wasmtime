;;! component_model_async = true

;; Here `$C.f` is blocked in a synchronous `waitable-set.wait` inside its
;; callback-lifted start function when `$A` requests cancellation. `f` then
;; wakes up, calls `task.return` and returns YIELD. Its callback should be
;; invoked with EVENT_NONE, not TASK_CANCELLED, and `$A`'s `subtask.cancel`
;; observes RETURNED.
(component
  (component $C
    (core module $libc (memory (export "mem") 1))
    (core instance $libc (instantiate $libc))
    (type $fut (future u32))
    (core func $future.read
      (canon future.read $fut async (memory (core memory $libc "mem"))))
    (core func $waitable-set.new (canon waitable-set.new))
    (core func $waitable-set.wait
      (canon waitable-set.wait (memory (core memory $libc "mem"))))
    (core func $waitable.join (canon waitable.join))
    (core func $task.return (canon task.return))
    (core module $m
      (import "" "future.read"
        (func $future.read (param i32 i32) (result i32)))
      (import "" "waitable-set.new" (func $waitable-set.new (result i32)))
      (import "" "waitable-set.wait"
        (func $waitable-set.wait (param i32 i32) (result i32)))
      (import "" "waitable.join" (func $waitable.join (param i32 i32)))
      (import "" "task.return" (func $task.return))

      (func (export "f") (param $fut i32) (result i32)
        (local $s i32)
        (if (i32.ne (call $future.read (local.get $fut) (i32.const 0))
              (i32.const -1 (; BLOCKED ;)))
          (then unreachable))
        (local.set $s (call $waitable-set.new))
        (call $waitable.join (local.get $fut) (local.get $s))
        ;; cancellation is requested while we're blocked here
        (drop (call $waitable-set.wait (local.get $s) (i32.const 8)))
        (call $waitable.join (local.get $fut) (i32.const 0))
        ;; return normally; the task is now RESOLVED
        (call $task.return)
        (i32.const 1 (; YIELD ;)))
      (func (export "f-cb") (param $ev i32) (param i32 i32) (result i32)
        ;; spec: EVENT_NONE (0); TASK_CANCELLED (6) must not be delivered
        (if (i32.ne (local.get $ev) (i32.const 0 (; EVENT_NONE ;)))
          (then unreachable))
        (i32.const 0 (; EXIT ;)))
    )
    (core instance $i (instantiate $m
      (with "" (instance
        (export "future.read" (func $future.read))
        (export "waitable-set.new" (func $waitable-set.new))
        (export "waitable-set.wait" (func $waitable-set.wait))
        (export "waitable.join" (func $waitable.join))
        (export "task.return" (func $task.return))
      ))
    ))
    (func (export "f") async (param "x" $fut)
      (canon lift (core func $i "f") async (callback (core func $i "f-cb"))))
  )
  (instance $c (instantiate $C))

  (component $A
    (type $fut (future u32))
    (import "f" (func $f async (param "x" $fut)))
    (core module $libc (memory (export "mem") 1))
    (core instance $libc (instantiate $libc))
    (core func $f
      (canon lower (func $f) async (memory (core memory $libc "mem"))))
    (core func $future.new (canon future.new $fut))
    (core func $future.write
      (canon future.write $fut async (memory (core memory $libc "mem"))))
    (core func $subtask.cancel (canon subtask.cancel))
    (core func $subtask.drop (canon subtask.drop))
    (core func $task.return (canon task.return))
    (core module $m
      (import "" "mem" (memory 1))
      (import "" "f" (func $f (param i32) (result i32)))
      (import "" "future.new" (func $future.new (result i64)))
      (import "" "future.write"
        (func $future.write (param i32 i32) (result i32)))
      (import "" "subtask.cancel"
        (func $subtask.cancel (param i32) (result i32)))
      (import "" "subtask.drop" (func $subtask.drop (param i32)))
      (import "" "task.return" (func $task.return))
      (global $n (mut i32) (i32.const 0))

      (func (export "run") (result i32)
        (local $tmp i64) (local $r i32) (local $w i32) (local $st i32)
        (local $h i32)
        (local.set $tmp (call $future.new))
        (local.set $r (i32.wrap_i64 (local.get $tmp)))
        (local.set $w
          (i32.wrap_i64 (i64.shr_u (local.get $tmp) (i64.const 32))))
        (local.set $st (call $f (local.get $r)))
        (if (i32.ne (i32.and (local.get $st) (i32.const 0xf))
              (i32.const 1 (; STARTED ;)))
          (then unreachable))
        (local.set $h (i32.shr_u (local.get $st) (i32.const 4)))
        ;; unblock f, then synchronously cancel it; f returns normally
        (drop (call $future.write (local.get $w) (i32.const 0)))
        (if (i32.ne (call $subtask.cancel (local.get $h))
              (i32.const 2 (; RETURNED ;)))
          (then unreachable))
        (call $subtask.drop (local.get $h))
        ;; yield a few times so f's callback gets a chance to run
        (i32.const 1 (; YIELD ;)))
      (func (export "cb") (param i32 i32 i32) (result i32)
        (global.set $n (i32.add (global.get $n) (i32.const 1)))
        (if (i32.lt_u (global.get $n) (i32.const 10))
          (then (return (i32.const 1 (; YIELD ;)))))
        (call $task.return)
        (i32.const 0 (; EXIT ;)))
    )
    (core instance $i (instantiate $m
      (with "" (instance
        (export "mem" (memory $libc "mem"))
        (export "f" (func $f))
        (export "future.new" (func $future.new))
        (export "future.write" (func $future.write))
        (export "subtask.cancel" (func $subtask.cancel))
        (export "subtask.drop" (func $subtask.drop))
        (export "task.return" (func $task.return))
      ))
    ))
    (func (export "run") async
      (canon lift (core func $i "run") async (callback (core func $i "cb"))))
  )
  (instance $a (instantiate $A (with "f" (func $c "f"))))
  (export "run" (func $a "run"))
)

(assert_return (invoke "run"))
