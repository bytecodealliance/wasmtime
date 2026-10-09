;;! component_model_async = true
;;! component_model_threading = true

;; The implicit thread of a callback-lifted task waits on set S (returning
;; `WAIT`). An explicit thread of the same task then completes a read whose
;; readable end is in S (waking the implicit thread up) and immediately takes
;; that event with `waitable-set.poll`. The implicit thread should keep waiting
;; and deadlock here.
(component
  (core module $libc
    (table (export "t") 1 funcref)
    (memory (export "m") 1))
  (core instance $libc (instantiate $libc))
  (core type $start-func-ty (func (param i32)))

  (type $f (future))
  (core func $future.new (canon future.new $f))
  (core func $future.read (canon future.read $f async))
  (core func $future.write (canon future.write $f async))
  (core func $waitable-set.new (canon waitable-set.new))
  (core func $waitable.join (canon waitable.join))
  (core func $waitable-set.poll
    (canon waitable-set.poll (memory (core memory $libc "m"))))
  (core func $thread.new-indirect
    (canon thread.new-indirect $start-func-ty (core table $libc "t")))
  (core func $thread.resume-later (canon thread.resume-later))
  (core func $task.return (canon task.return))

  (core module $m
    (import "" "m" (memory 1))
    (import "" "t" (table 1 funcref))
    (import "" "future.new" (func $future.new (result i64)))
    (import "" "future.read" (func $future.read (param i32 i32) (result i32)))
    (import "" "future.write" (func $future.write (param i32 i32) (result i32)))
    (import "" "waitable-set.new" (func $ws.new (result i32)))
    (import "" "waitable.join" (func $join (param i32 i32)))
    (import "" "waitable-set.poll" (func $poll (param i32 i32) (result i32)))
    (import "" "thread.new-indirect"
      (func $thread.new-indirect (param i32 i32) (result i32)))
    (import "" "thread.resume-later" (func $thread.resume-later (param i32)))
    (import "" "task.return" (func $task.return))

    (global $w (mut i32) (i32.const 0))
    (global $s (mut i32) (i32.const 0))

    (func $thread (param i32)
      ;; complete the pending read, waking the implicit thread
      (if (i32.ne (call $future.write (global.get $w) (i32.const 0))
            (i32.const 0 (; COMPLETED ;)))
        (then unreachable))
      ;; ... and take the event before the implicit thread runs
      (if (i32.ne (call $poll (global.get $s) (i32.const 0))
            (i32.const 4 (; FUTURE_READ ;)))
        (then unreachable)))
    (elem (i32.const 0) func $thread)

    (func (export "run") (result i32)
      (local $tmp i64) (local $r i32)
      (local.set $tmp (call $future.new))
      (local.set $r (i32.wrap_i64 (local.get $tmp)))
      (global.set $w (i32.wrap_i64 (i64.shr_u (local.get $tmp) (i64.const 32))))
      (if (i32.ne (call $future.read (local.get $r) (i32.const 0))
            (i32.const -1 (; BLOCKED ;)))
        (then unreachable))
      (global.set $s (call $ws.new))
      (call $join (local.get $r) (global.get $s))
      (call $thread.resume-later
        (call $thread.new-indirect (i32.const 0) (i32.const 0)))
      ;; CALLBACK_CODE_WAIT | (set << 4)
      (i32.or (i32.const 2) (i32.shl (global.get $s) (i32.const 4))))
    (func (export "cb") (param i32 i32 i32) (result i32) unreachable)
  )

  (core instance $i (instantiate $m (with "" (instance
    (export "m" (memory $libc "m"))
    (export "t" (table $libc "t"))
    (export "future.new" (func $future.new))
    (export "future.read" (func $future.read))
    (export "future.write" (func $future.write))
    (export "waitable-set.new" (func $waitable-set.new))
    (export "waitable.join" (func $waitable.join))
    (export "waitable-set.poll" (func $waitable-set.poll))
    (export "thread.new-indirect" (func $thread.new-indirect))
    (export "thread.resume-later" (func $thread.resume-later))
    (export "task.return" (func $task.return))))))

  (func (export "run") async
    (canon lift (core func $i "run") async (callback (core func $i "cb"))))
)
(assert_trap (invoke "run") "deadlock detected")

;; Same as above, but the explicit thread drops S after moving the readable end
;; out of it. This should trap since the implicit thread is still waiting.
(component
  (core module $libc
    (table (export "t") 1 funcref))
  (core instance $libc (instantiate $libc))
  (core type $start-func-ty (func (param i32)))

  (type $f (future))
  (core func $future.new (canon future.new $f))
  (core func $future.read (canon future.read $f async))
  (core func $future.write (canon future.write $f async))
  (core func $waitable-set.new (canon waitable-set.new))
  (core func $waitable-set.drop (canon waitable-set.drop))
  (core func $waitable.join (canon waitable.join))
  (core func $thread.new-indirect
    (canon thread.new-indirect $start-func-ty (core table $libc "t")))
  (core func $thread.resume-later (canon thread.resume-later))
  (core func $task.return (canon task.return))

  (core module $m
    (import "" "t" (table 1 funcref))
    (import "" "future.new" (func $future.new (result i64)))
    (import "" "future.read" (func $future.read (param i32 i32) (result i32)))
    (import "" "future.write" (func $future.write (param i32 i32) (result i32)))
    (import "" "waitable-set.new" (func $ws.new (result i32)))
    (import "" "waitable-set.drop" (func $ws.drop (param i32)))
    (import "" "waitable.join" (func $join (param i32 i32)))
    (import "" "thread.new-indirect"
      (func $thread.new-indirect (param i32 i32) (result i32)))
    (import "" "thread.resume-later" (func $thread.resume-later (param i32)))
    (import "" "task.return" (func $task.return))

    (global $r (mut i32) (i32.const 0))
    (global $w (mut i32) (i32.const 0))
    (global $s (mut i32) (i32.const 0))

    (func $thread (param i32)
      ;; complete the pending read, waking the implicit thread
      (if (i32.ne (call $future.write (global.get $w) (i32.const 0))
            (i32.const 0 (; COMPLETED ;)))
        (then unreachable))
      ;; the implicit thread is still waiting on S, so it can't be dropped
      (call $join (global.get $r) (i32.const 0))
      (call $ws.drop (global.get $s)))
    (elem (i32.const 0) func $thread)

    (func (export "run") (result i32)
      (local $tmp i64)
      (local.set $tmp (call $future.new))
      (global.set $r (i32.wrap_i64 (local.get $tmp)))
      (global.set $w (i32.wrap_i64 (i64.shr_u (local.get $tmp) (i64.const 32))))
      (if (i32.ne (call $future.read (global.get $r) (i32.const 0))
            (i32.const -1 (; BLOCKED ;)))
        (then unreachable))
      (global.set $s (call $ws.new))
      (call $join (global.get $r) (global.get $s))
      (call $thread.resume-later
        (call $thread.new-indirect (i32.const 0) (i32.const 0)))
      ;; CALLBACK_CODE_WAIT | (set << 4)
      (i32.or (i32.const 2) (i32.shl (global.get $s) (i32.const 4))))
    (func (export "cb") (param i32 i32 i32) (result i32) unreachable)
  )

  (core instance $i (instantiate $m (with "" (instance
    (export "t" (table $libc "t"))
    (export "future.new" (func $future.new))
    (export "future.read" (func $future.read))
    (export "future.write" (func $future.write))
    (export "waitable-set.new" (func $waitable-set.new))
    (export "waitable-set.drop" (func $waitable-set.drop))
    (export "waitable.join" (func $waitable.join))
    (export "thread.new-indirect" (func $thread.new-indirect))
    (export "thread.resume-later" (func $thread.resume-later))
    (export "task.return" (func $task.return))))))

  (func (export "run") async
    (canon lift (core func $i "run") async (callback (core func $i "cb"))))
)
(assert_trap (invoke "run") "cannot drop waitable set with waiters")

;; The reverse: an explicit thread blocks in `waitable-set.wait` on S, and the
;; implicit thread (using a callback) completes a read in S, waking the explicit
;; thread up, and then takes the event itself with `waitable-set.poll`. The
;; explicit thread should just keep waiting.
(component
  (core module $libc
    (table (export "t") 1 funcref)
    (memory (export "m") 1))
  (core instance $libc (instantiate $libc))
  (core type $start-func-ty (func (param i32)))

  (type $f (future))
  (core func $future.new (canon future.new $f))
  (core func $future.read (canon future.read $f async))
  (core func $future.write (canon future.write $f async))
  (core func $waitable-set.new (canon waitable-set.new))
  (core func $waitable.join (canon waitable.join))
  (core func $waitable-set.wait
    (canon waitable-set.wait (memory (core memory $libc "m"))))
  (core func $waitable-set.poll
    (canon waitable-set.poll (memory (core memory $libc "m"))))
  (core func $thread.new-indirect
    (canon thread.new-indirect $start-func-ty (core table $libc "t")))
  (core func $thread.resume-later (canon thread.resume-later))
  (core func $task.return (canon task.return))

  (core module $m
    (import "" "m" (memory 1))
    (import "" "t" (table 1 funcref))
    (import "" "future.new" (func $future.new (result i64)))
    (import "" "future.read" (func $future.read (param i32 i32) (result i32)))
    (import "" "future.write" (func $future.write (param i32 i32) (result i32)))
    (import "" "waitable-set.new" (func $ws.new (result i32)))
    (import "" "waitable.join" (func $join (param i32 i32)))
    (import "" "waitable-set.wait" (func $wait (param i32 i32) (result i32)))
    (import "" "waitable-set.poll" (func $poll (param i32 i32) (result i32)))
    (import "" "thread.new-indirect"
      (func $thread.new-indirect (param i32 i32) (result i32)))
    (import "" "thread.resume-later" (func $thread.resume-later (param i32)))
    (import "" "task.return" (func $task.return))

    (global $w (mut i32) (i32.const 0))
    (global $s (mut i32) (i32.const 0))
    (global $state (mut i32) (i32.const 0))
    (global $woke (mut i32) (i32.const 0))

    (func $thread (param i32)
      (drop (call $wait (global.get $s) (i32.const 16)))
      (global.set $woke (i32.const 1)))
    (elem (i32.const 0) func $thread)

    (func (export "run") (result i32)
      (local $tmp i64) (local $r i32)
      (local.set $tmp (call $future.new))
      (local.set $r (i32.wrap_i64 (local.get $tmp)))
      (global.set $w (i32.wrap_i64 (i64.shr_u (local.get $tmp) (i64.const 32))))
      (if (i32.ne (call $future.read (local.get $r) (i32.const 0))
            (i32.const -1 (; BLOCKED ;)))
        (then unreachable))
      (global.set $s (call $ws.new))
      (call $join (local.get $r) (global.get $s))
      ;; let the explicit thread block in `waitable-set.wait`
      (call $thread.resume-later
        (call $thread.new-indirect (i32.const 0) (i32.const 0)))
      (i32.const 1 (; YIELD ;)))
    (func (export "cb") (param i32 i32 i32) (result i32)
      (if (i32.eqz (global.get $state))
        (then
          (global.set $state (i32.const 1))
          ;; complete the read; this wakes the explicit thread
          (if (i32.ne (call $future.write (global.get $w) (i32.const 0))
                (i32.const 0 (; COMPLETED ;)))
            (then unreachable))
          ;; ... but take the event before it runs
          (if (i32.ne (call $poll (global.get $s) (i32.const 0))
                (i32.const 4 (; FUTURE_READ ;)))
            (then unreachable))
          ;; let the explicit thread run
          (return (i32.const 1 (; YIELD ;)))))
      ;; the explicit thread must still be waiting
      (if (global.get $woke) (then unreachable))
      (call $task.return)
      (i32.const 0 (; EXIT ;)))
  )

  (core instance $i (instantiate $m (with "" (instance
    (export "m" (memory $libc "m"))
    (export "t" (table $libc "t"))
    (export "future.new" (func $future.new))
    (export "future.read" (func $future.read))
    (export "future.write" (func $future.write))
    (export "waitable-set.new" (func $waitable-set.new))
    (export "waitable.join" (func $waitable.join))
    (export "waitable-set.wait" (func $waitable-set.wait))
    (export "waitable-set.poll" (func $waitable-set.poll))
    (export "thread.new-indirect" (func $thread.new-indirect))
    (export "thread.resume-later" (func $thread.resume-later))
    (export "task.return" (func $task.return))))))

  (func (export "run") async
    (canon lift (core func $i "run") async (callback (core func $i "cb"))))
)
(assert_return (invoke "run"))

;; A callback returning `YIELD` must not leave an event behind which another
;; thread of the same task could take: here an explicit thread polls an empty
;; set (which must return `EVENT_NONE` without affecting anything else) before
;; the callback receives its `EVENT_NONE`.
(component
  (core module $libc
    (table (export "t") 1 funcref)
    (memory (export "m") 1))
  (core instance $libc (instantiate $libc))
  (core type $start-func-ty (func (param i32)))

  (core func $waitable-set.new (canon waitable-set.new))
  (core func $waitable-set.poll
    (canon waitable-set.poll (memory (core memory $libc "m"))))
  (core func $thread.new-indirect
    (canon thread.new-indirect $start-func-ty (core table $libc "t")))
  (core func $thread.resume-later (canon thread.resume-later))
  (core func $task.return (canon task.return))

  (core module $m
    (import "" "t" (table 1 funcref))
    (import "" "waitable-set.new" (func $ws.new (result i32)))
    (import "" "waitable-set.poll" (func $poll (param i32 i32) (result i32)))
    (import "" "thread.new-indirect"
      (func $thread.new-indirect (param i32 i32) (result i32)))
    (import "" "thread.resume-later" (func $thread.resume-later (param i32)))
    (import "" "task.return" (func $task.return))

    (global $polled (mut i32) (i32.const 0))

    (func $thread (param i32)
      (if (i32.ne (call $poll (call $ws.new) (i32.const 0))
            (i32.const 0 (; NONE ;)))
        (then unreachable))
      (global.set $polled (i32.const 1)))
    (elem (i32.const 0) func $thread)

    (func (export "run") (result i32)
      (call $thread.resume-later
        (call $thread.new-indirect (i32.const 0) (i32.const 0)))
      (i32.const 1 (; YIELD ;)))
    (func (export "cb") (param $event i32) (param i32 i32) (result i32)
      (if (i32.ne (local.get $event) (i32.const 0 (; NONE ;)))
        (then unreachable))
      (if (i32.eqz (global.get $polled)) (then unreachable))
      (call $task.return)
      (i32.const 0 (; EXIT ;)))
  )

  (core instance $i (instantiate $m (with "" (instance
    (export "t" (table $libc "t"))
    (export "waitable-set.new" (func $waitable-set.new))
    (export "waitable-set.poll" (func $waitable-set.poll))
    (export "thread.new-indirect" (func $thread.new-indirect))
    (export "thread.resume-later" (func $thread.resume-later))
    (export "task.return" (func $task.return))))))

  (func (export "run") async
    (canon lift (core func $i "run") async (callback (core func $i "cb"))))
)
(assert_return (invoke "run"))
