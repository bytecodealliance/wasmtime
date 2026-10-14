;;! component_model_async = true

;; Per the spec's `canon_lift` callback loop:
;;
;;   while code != CallbackCode.EXIT:
;;     if thread.task.deliver_pending_cancel():
;;       event = (EventCode.TASK_CANCELLED, 0, 0)
;;     else:
;;       ...
;;       inst.exclusive_thread = None
;;       ...
;;
;; when a callback-lifted task returns WAIT or YIELD while a cancellation is
;; pending, the callback is immediately re-invoked with TASK_CANCELLED without
;; releasing the instance's exclusive lock, so no other task may enter the
;; instance in between.

;; The start function of `$C.f` blocks in a synchronous `waitable-set.wait`,
;; holding the exclusive lock. Meanwhile `$A` starts `$C.g` (which must wait
;; for the lock, so is STARTING), unblocks `f`'s wait and synchronously cancels
;; `f`. When `f`'s start function then returns WAIT, TASK_CANCELLED must be
;; delivered to `f` before `g` starts.
(component
  (component $C
    (core module $libc (memory (export "mem") 1))
    (core instance $libc (instantiate $libc))
    (type $fut (future u32))
    (core func $future.read (canon future.read $fut async (memory (core memory $libc "mem"))))
    (core func $waitable-set.new (canon waitable-set.new))
    (core func $waitable-set.wait (canon waitable-set.wait (memory (core memory $libc "mem"))))
    (core func $waitable.join (canon waitable.join))
    (core func $task.cancel (canon task.cancel))
    (core func $task.return (canon task.return))
    (core module $m
      (import "" "future.read" (func $future.read (param i32 i32) (result i32)))
      (import "" "waitable-set.new" (func $waitable-set.new (result i32)))
      (import "" "waitable-set.wait" (func $waitable-set.wait (param i32 i32) (result i32)))
      (import "" "waitable.join" (func $waitable.join (param i32 i32)))
      (import "" "task.cancel" (func $task.cancel))
      (import "" "task.return" (func $task.return))
      (global $g_ran (mut i32) (i32.const 0))

      (func (export "f") (param $fut i32) (result i32)
        (local $s i32)
        ;; async read (BLOCKED), then a synchronous `waitable-set.wait` from
        ;; within the start function: blocks while holding the exclusive lock
        (if (i32.ne (call $future.read (local.get $fut) (i32.const 0)) (i32.const -1))
          (then unreachable))
        (local.set $s (call $waitable-set.new))
        (call $waitable.join (local.get $fut) (local.get $s))
        (drop (call $waitable-set.wait (local.get $s) (i32.const 8)))
        (call $waitable.join (local.get $fut) (i32.const 0))
        ;; return WAIT on a fresh (empty) set
        (i32.or (i32.shl (call $waitable-set.new) (i32.const 4)) (i32.const 2)))
      (func (export "f-cb") (param $ev i32) (param i32 i32) (result i32)
        (if (i32.ne (local.get $ev) (i32.const 6)) (then unreachable))
        ;; cancellation is delivered before `g` can enter
        (if (global.get $g_ran) (then unreachable))
        (call $task.cancel)
        (i32.const 0))

      (func (export "g") (result i32)
        (global.set $g_ran (i32.const 1))
        (call $task.return)
        (i32.const 0))
      (func (export "g-cb") (param i32 i32 i32) (result i32) unreachable)
    )
    (core instance $i (instantiate $m
      (with "" (instance
        (export "future.read" (func $future.read))
        (export "waitable-set.new" (func $waitable-set.new))
        (export "waitable-set.wait" (func $waitable-set.wait))
        (export "waitable.join" (func $waitable.join))
        (export "task.cancel" (func $task.cancel))
        (export "task.return" (func $task.return))
      ))
    ))
    (func (export "f") async (param "x" $fut)
      (canon lift (core func $i "f") async (callback (core func $i "f-cb"))))
    (func (export "g") async
      (canon lift (core func $i "g") async (callback (core func $i "g-cb"))))
  )
  (instance $c (instantiate $C))

  (component $A
    (type $fut (future u32))
    (import "f" (func $f async (param "x" $fut)))
    (import "g" (func $g async))
    (core module $libc (memory (export "mem") 1))
    (core instance $libc (instantiate $libc))
    (core func $f (canon lower (func $f) async (memory (core memory $libc "mem"))))
    (core func $g (canon lower (func $g) async (memory (core memory $libc "mem"))))
    (core func $future.new (canon future.new $fut))
    (core func $future.write (canon future.write $fut async (memory (core memory $libc "mem"))))
    (core func $subtask.cancel (canon subtask.cancel))
    (core func $subtask.drop (canon subtask.drop))
    (core func $waitable-set.new (canon waitable-set.new))
    (core func $waitable.join (canon waitable.join))
    (core func $task.return (canon task.return))
    (core module $m
      (import "libc" "mem" (memory 1))
      (import "" "f" (func $f (param i32) (result i32)))
      (import "" "g" (func $g (result i32)))
      (import "" "future.new" (func $future.new (result i64)))
      (import "" "future.write" (func $future.write (param i32 i32) (result i32)))
      (import "" "subtask.cancel" (func $subtask.cancel (param i32) (result i32)))
      (import "" "subtask.drop" (func $subtask.drop (param i32)))
      (import "" "waitable-set.new" (func $waitable-set.new (result i32)))
      (import "" "waitable.join" (func $waitable.join (param i32 i32)))
      (import "" "task.return" (func $task.return))
      (global $set (mut i32) (i32.const 0))

      (func (export "run") (result i32)
        (local $tmp i64) (local $r i32) (local $w i32)
        (local $st i32) (local $h1 i32) (local $h2 i32)
        (local.set $tmp (call $future.new))
        (local.set $r (i32.wrap_i64 (local.get $tmp)))
        (local.set $w (i32.wrap_i64 (i64.shr_u (local.get $tmp) (i64.const 32))))

        ;; f blocks in its start function => STARTED
        (local.set $st (call $f (local.get $r)))
        (if (i32.ne (i32.and (local.get $st) (i32.const 0xf)) (i32.const 1))
          (then unreachable))
        (local.set $h1 (i32.shr_u (local.get $st) (i32.const 4)))

        ;; g can't enter $C (f holds the exclusive lock) => STARTING
        (local.set $st (call $g))
        (if (i32.ne (i32.and (local.get $st) (i32.const 0xf)) (i32.const 0))
          (then unreachable))
        (local.set $h2 (i32.shr_u (local.get $st) (i32.const 4)))

        ;; unblock f's wait
        (i32.store (i32.const 0) (i32.const 42))
        (drop (call $future.write (local.get $w) (i32.const 0)))

        ;; synchronously cancel f => RETURN_CANCELLED (4)
        (if (i32.ne (call $subtask.cancel (local.get $h1)) (i32.const 4))
          (then unreachable))
        (call $subtask.drop (local.get $h1))

        (global.set $set (call $waitable-set.new))
        (call $waitable.join (local.get $h2) (global.get $set))
        (i32.or (i32.shl (global.get $set) (i32.const 4)) (i32.const 2)))

      (func (export "cb") (param $ev i32) (param $h i32) (param $status i32) (result i32)
        (if (i32.ne (local.get $ev) (i32.const 1)) (then unreachable))
        ;; RETURNED (2) is terminal
        (if (i32.eq (local.get $status) (i32.const 2))
          (then
            (call $subtask.drop (local.get $h))
            (call $task.return)
            (return (i32.const 0))))
        (i32.or (i32.shl (global.get $set) (i32.const 4)) (i32.const 2)))
    )
    (core instance $i (instantiate $m
      (with "libc" (instance $libc))
      (with "" (instance
        (export "f" (func $f))
        (export "g" (func $g))
        (export "future.new" (func $future.new))
        (export "future.write" (func $future.write))
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
  (instance $a (instantiate $A
    (with "f" (func $c "f"))
    (with "g" (func $c "g"))))
  (export "run" (func $a "run"))
)

(assert_return (invoke "run"))

;; Same as above, but `f` blocks in its callback rather than its start
;; function, and the callback then returns YIELD.
(component
  (component $C
    (core module $libc (memory (export "mem") 1))
    (core instance $libc (instantiate $libc))
    (type $fut (future u32))
    (core func $future.read (canon future.read $fut async (memory (core memory $libc "mem"))))
    (core func $waitable-set.new (canon waitable-set.new))
    (core func $waitable-set.wait (canon waitable-set.wait (memory (core memory $libc "mem"))))
    (core func $waitable.join (canon waitable.join))
    (core func $task.cancel (canon task.cancel))
    (core func $task.return (canon task.return))
    (core module $m
      (import "" "future.read" (func $future.read (param i32 i32) (result i32)))
      (import "" "waitable-set.new" (func $waitable-set.new (result i32)))
      (import "" "waitable-set.wait" (func $waitable-set.wait (param i32 i32) (result i32)))
      (import "" "waitable.join" (func $waitable.join (param i32 i32)))
      (import "" "task.cancel" (func $task.cancel))
      (import "" "task.return" (func $task.return))
      (global $g_ran (mut i32) (i32.const 0))
      (global $fut (mut i32) (i32.const 0))
      (global $f_state (mut i32) (i32.const 0))

      (func (export "f") (param $fut i32) (result i32)
        (global.set $fut (local.get $fut))
        ;; YIELD
        (i32.const 1))
      (func (export "f-cb") (param $ev i32) (param i32 i32) (result i32)
        (local $s i32)
        (if (i32.eqz (global.get $f_state))
          (then
            (global.set $f_state (i32.const 1))
            (if (i32.ne (local.get $ev) (i32.const 0)) (then unreachable))
            ;; async read (BLOCKED), then a synchronous `waitable-set.wait`
            ;; from within the callback: blocks while holding the exclusive
            ;; lock
            (if (i32.ne (call $future.read (global.get $fut) (i32.const 0)) (i32.const -1))
              (then unreachable))
            (local.set $s (call $waitable-set.new))
            (call $waitable.join (global.get $fut) (local.get $s))
            (drop (call $waitable-set.wait (local.get $s) (i32.const 8)))
            (call $waitable.join (global.get $fut) (i32.const 0))
            ;; YIELD
            (return (i32.const 1))))
        (if (i32.ne (local.get $ev) (i32.const 6)) (then unreachable))
        ;; cancellation is delivered before `g` can enter
        (if (global.get $g_ran) (then unreachable))
        (call $task.cancel)
        (i32.const 0))

      (func (export "g") (result i32)
        (global.set $g_ran (i32.const 1))
        (call $task.return)
        (i32.const 0))
      (func (export "g-cb") (param i32 i32 i32) (result i32) unreachable)
    )
    (core instance $i (instantiate $m
      (with "" (instance
        (export "future.read" (func $future.read))
        (export "waitable-set.new" (func $waitable-set.new))
        (export "waitable-set.wait" (func $waitable-set.wait))
        (export "waitable.join" (func $waitable.join))
        (export "task.cancel" (func $task.cancel))
        (export "task.return" (func $task.return))
      ))
    ))
    (func (export "f") async (param "x" $fut)
      (canon lift (core func $i "f") async (callback (core func $i "f-cb"))))
    (func (export "g") async
      (canon lift (core func $i "g") async (callback (core func $i "g-cb"))))
  )
  (instance $c (instantiate $C))

  (component $A
    (type $fut (future u32))
    (import "f" (func $f async (param "x" $fut)))
    (import "g" (func $g async))
    (core module $libc (memory (export "mem") 1))
    (core instance $libc (instantiate $libc))
    (core func $f (canon lower (func $f) async (memory (core memory $libc "mem"))))
    (core func $g (canon lower (func $g) async (memory (core memory $libc "mem"))))
    (core func $future.new (canon future.new $fut))
    (core func $future.write (canon future.write $fut async (memory (core memory $libc "mem"))))
    (core func $subtask.cancel (canon subtask.cancel))
    (core func $subtask.drop (canon subtask.drop))
    (core func $waitable-set.new (canon waitable-set.new))
    (core func $waitable.join (canon waitable.join))
    (core func $task.return (canon task.return))
    (core module $m
      (import "libc" "mem" (memory 1))
      (import "" "f" (func $f (param i32) (result i32)))
      (import "" "g" (func $g (result i32)))
      (import "" "future.new" (func $future.new (result i64)))
      (import "" "future.write" (func $future.write (param i32 i32) (result i32)))
      (import "" "subtask.cancel" (func $subtask.cancel (param i32) (result i32)))
      (import "" "subtask.drop" (func $subtask.drop (param i32)))
      (import "" "waitable-set.new" (func $waitable-set.new (result i32)))
      (import "" "waitable.join" (func $waitable.join (param i32 i32)))
      (import "" "task.return" (func $task.return))
      (global $set (mut i32) (i32.const 0))
      (global $w (mut i32) (i32.const 0))
      (global $h1 (mut i32) (i32.const 0))
      (global $state (mut i32) (i32.const 0))

      (func (export "run") (result i32)
        (local $tmp i64) (local $st i32)
        (local.set $tmp (call $future.new))
        (global.set $w (i32.wrap_i64 (i64.shr_u (local.get $tmp) (i64.const 32))))

        ;; f yields from its start function => STARTED
        (local.set $st (call $f (i32.wrap_i64 (local.get $tmp))))
        (if (i32.ne (i32.and (local.get $st) (i32.const 0xf)) (i32.const 1))
          (then unreachable))
        (global.set $h1 (i32.shr_u (local.get $st) (i32.const 4)))

        ;; YIELD so that f's callback runs and blocks. The spec lets either
        ;; yielding task resume first; this test expects f's, which yielded
        ;; first, to run before this callback is called again.
        (i32.const 1))

      (func (export "cb") (param $ev i32) (param $h i32) (param $status i32) (result i32)
        (local $st i32) (local $h2 i32)
        (if (i32.eqz (global.get $state))
          (then
            (global.set $state (i32.const 1))
            (if (i32.ne (local.get $ev) (i32.const 0)) (then unreachable))

            ;; g can't enter $C (f holds the exclusive lock) => STARTING
            (local.set $st (call $g))
            (if (i32.ne (i32.and (local.get $st) (i32.const 0xf)) (i32.const 0))
              (then unreachable))
            (local.set $h2 (i32.shr_u (local.get $st) (i32.const 4)))

            ;; unblock f's wait
            (i32.store (i32.const 0) (i32.const 42))
            (drop (call $future.write (global.get $w) (i32.const 0)))

            ;; synchronously cancel f => RETURN_CANCELLED (4)
            (if (i32.ne (call $subtask.cancel (global.get $h1)) (i32.const 4))
              (then unreachable))
            (call $subtask.drop (global.get $h1))

            (global.set $set (call $waitable-set.new))
            (call $waitable.join (local.get $h2) (global.get $set))
            (return (i32.or (i32.shl (global.get $set) (i32.const 4)) (i32.const 2)))))

        (if (i32.ne (local.get $ev) (i32.const 1)) (then unreachable))
        ;; RETURNED (2) is terminal
        (if (i32.eq (local.get $status) (i32.const 2))
          (then
            (call $subtask.drop (local.get $h))
            (call $task.return)
            (return (i32.const 0))))
        (i32.or (i32.shl (global.get $set) (i32.const 4)) (i32.const 2)))
    )
    (core instance $i (instantiate $m
      (with "libc" (instance $libc))
      (with "" (instance
        (export "f" (func $f))
        (export "g" (func $g))
        (export "future.new" (func $future.new))
        (export "future.write" (func $future.write))
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
  (instance $a (instantiate $A
    (with "f" (func $c "f"))
    (with "g" (func $c "g"))))
  (export "run" (func $a "run"))
)

(assert_return (invoke "run"))
