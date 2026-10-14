;;! component_model_async = true

;; Two tasks in `$X` wait on the same waitable set: `first-waiter`, then
;; `second-waiter`. `$C` makes an event ready in the set, which wakes
;; `first-waiter`, and then cancels `first-waiter` before it runs.
;; `first-waiter` receives TASK_CANCELLED instead of the set's event, so the
;; event stays in the set and `second-waiter` must be woken up to receive it.
(component
  (type $FT (future u32))

  (component $X
    (type $FT (future u32))
    (core module $libc (memory (export "memory") 1))
    (core instance $libc (instantiate $libc))
    (core func $waitable-set.new (canon waitable-set.new))
    (core func $waitable.join (canon waitable.join))
    (core func $task.return (canon task.return))
    (core func $task.cancel (canon task.cancel))
    (core func $future.read
      (canon future.read $FT async (memory (core memory $libc "memory"))))
    (core module $m
      (import "" "waitable-set.new" (func $waitable-set.new (result i32)))
      (import "" "waitable.join" (func $waitable.join (param i32 i32)))
      (import "" "task.return" (func $task.return))
      (import "" "task.cancel" (func $task.cancel))
      (import "" "future.read" (func $future.read (param i32 i32) (result i32)))
      (global $set (mut i32) (i32.const 0))

      ;; Start a read of `$f`, join it to a new set, and wait on that set.
      (func (export "first-waiter") (param $f i32) (result i32)
        (global.set $set (call $waitable-set.new))
        (if (i32.ne (call $future.read (local.get $f) (i32.const 0))
              (i32.const -1 (; BLOCKED ;)))
          (then unreachable))
        (call $waitable.join (local.get $f) (global.get $set))
        (i32.or (i32.const 2 (; WAIT ;))
          (i32.shl (global.get $set) (i32.const 4))))
      (func (export "first-waiter-cb")
        (param $event i32) (param $handle i32) (param $payload i32) (result i32)
        (if (i32.ne (local.get $event) (i32.const 6 (; TASK_CANCELLED ;)))
          (then unreachable))
        (call $task.cancel)
        (i32.const 0 (; EXIT ;)))

      ;; Wait on `first-waiter`'s set and expect the event for its read.
      (func (export "second-waiter") (result i32)
        (i32.or (i32.const 2 (; WAIT ;))
          (i32.shl (global.get $set) (i32.const 4))))
      (func (export "second-waiter-cb")
        (param $event i32) (param $handle i32) (param $payload i32) (result i32)
        (if (i32.ne (local.get $event) (i32.const 4 (; FUTURE_READ ;)))
          (then unreachable))
        (call $task.return)
        (i32.const 0 (; EXIT ;)))
    )
    (core instance $i (instantiate $m (with "" (instance
      (export "waitable-set.new" (func $waitable-set.new))
      (export "waitable.join" (func $waitable.join))
      (export "task.return" (func $task.return))
      (export "task.cancel" (func $task.cancel))
      (export "future.read" (func $future.read))))))
    (func (export "first-waiter") async (param "f" $FT)
      (canon lift (core func $i "first-waiter") async
        (callback (core func $i "first-waiter-cb"))))
    (func (export "second-waiter") async
      (canon lift (core func $i "second-waiter") async
        (callback (core func $i "second-waiter-cb")))))

  (component $C
    (type $FT (future u32))
    (import "first-waiter" (func $first-waiter async (param "f" $FT)))
    (import "second-waiter" (func $second-waiter async))
    (core module $libc (memory (export "memory") 1))
    (core instance $libc (instantiate $libc))
    (core func $first-waiter (canon lower (func $first-waiter) async))
    (core func $second-waiter (canon lower (func $second-waiter) async))
    (core func $waitable-set.new (canon waitable-set.new))
    (core func $waitable.join (canon waitable.join))
    (core func $task.return (canon task.return (result u32)))
    (core func $subtask.cancel (canon subtask.cancel))
    (core func $future.new (canon future.new $FT))
    (core func $future.write
      (canon future.write $FT async (memory (core memory $libc "memory"))))
    (core module $m
      (import "" "first-waiter" (func $first-waiter (param i32) (result i32)))
      (import "" "second-waiter" (func $second-waiter (result i32)))
      (import "" "waitable-set.new" (func $waitable-set.new (result i32)))
      (import "" "waitable.join" (func $waitable.join (param i32 i32)))
      (import "" "task.return" (func $task.return (param i32)))
      (import "" "subtask.cancel" (func $subtask.cancel (param i32) (result i32)))
      (import "" "future.new" (func $future.new (result i64)))
      (import "" "future.write" (func $future.write (param i32 i32) (result i32)))

      (func (export "run") (result i32)
        (local $futures i64) (local $first-waiter i32) (local $second-waiter i32)
        (local $set i32)
        (local.set $futures (call $future.new))

        ;; Start `first-waiter` and then `second-waiter`, both of which block.
        (local.set $first-waiter
          (call $first-waiter (i32.wrap_i64 (local.get $futures))))
        (if (i32.ne (i32.and (local.get $first-waiter) (i32.const 0xf))
              (i32.const 1 (; STARTED ;)))
          (then unreachable))
        (local.set $second-waiter (call $second-waiter))
        (if (i32.ne (i32.and (local.get $second-waiter) (i32.const 0xf))
              (i32.const 1 (; STARTED ;)))
          (then unreachable))

        ;; Complete `first-waiter`'s read, which wakes it, then cancel it
        ;; before it runs.
        (if (i32.ne (call $future.write
                      (i32.wrap_i64 (i64.shr_u (local.get $futures) (i64.const 32)))
                      (i32.const 0))
              (i32.const 0 (; COMPLETED ;)))
          (then unreachable))
        (if (i32.ne (call $subtask.cancel
                      (i32.shr_u (local.get $first-waiter) (i32.const 4)))
              (i32.const 4 (; CANCELLED_BEFORE_RETURNED ;)))
          (then unreachable))

        ;; Wait for `second-waiter` to finish.
        (local.set $set (call $waitable-set.new))
        (call $waitable.join
          (i32.shr_u (local.get $second-waiter) (i32.const 4))
          (local.get $set))
        (i32.or (i32.const 2 (; WAIT ;))
          (i32.shl (local.get $set) (i32.const 4))))
      (func (export "run-cb")
        (param $event i32) (param $handle i32) (param $payload i32) (result i32)
        (if (i32.ne (local.get $event) (i32.const 1 (; SUBTASK ;)))
          (then unreachable))
        (if (i32.ne (local.get $payload) (i32.const 2 (; RETURNED ;)))
          (then unreachable))
        (call $task.return (i32.const 42))
        (i32.const 0 (; EXIT ;))))
    (core instance $i (instantiate $m (with "" (instance
      (export "first-waiter" (func $first-waiter))
      (export "second-waiter" (func $second-waiter))
      (export "waitable-set.new" (func $waitable-set.new))
      (export "waitable.join" (func $waitable.join))
      (export "task.return" (func $task.return))
      (export "subtask.cancel" (func $subtask.cancel))
      (export "future.new" (func $future.new))
      (export "future.write" (func $future.write))))))
    (func (export "run") async (result u32)
      (canon lift (core func $i "run") async
        (callback (core func $i "run-cb")))))

  (instance $x (instantiate $X))
  (instance $c (instantiate $C
    (with "first-waiter" (func $x "first-waiter"))
    (with "second-waiter" (func $x "second-waiter"))))
  (export "run" (func $c "run"))
)

(assert_return (invoke "run") (u32.const 42))
