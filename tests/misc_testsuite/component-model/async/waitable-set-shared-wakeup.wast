;;! component_model_async = true

;; Two threads of the same instance $X wait on the same waitable set $S:
;;
;; - t1: a callback-lifted task which returned WAIT on $S, and
;; - t2: a sync-lifted, async-typed task (so it holds $X's exclusive lock)
;;   blocked in `waitable-set.wait` on $S.
;;
;; When an event arrives on $S, the spec makes every waiter whose readiness
;; predicate holds runnable: t2 (`WaitableSet.wait` only needs a pending event)
;; is ready, t1 (`wait_from_callback` additionally needs the exclusive lock to
;; be free) is not. So t2 runs, consumes the event, starts another host call
;; that it also joins to $S and returns, releasing the lock; t1 then gets the
;; second event and returns.
(component
  (import "host" (instance $host
    (export "return-two-slowly" (func async (result s32)))))

  (component $X
    (import "slow" (func $slow async (result s32)))
    (core module $libc (memory (export "mem") 1))
    (core instance $libc (instantiate $libc))
    (core func $slow
      (canon lower (func $slow) async (memory (core memory $libc "mem"))))
    (core func $waitable-set.new (canon waitable-set.new))
    (core func $waitable-set.wait
      (canon waitable-set.wait (memory (core memory $libc "mem"))))
    (core func $waitable.join (canon waitable.join))
    (core func $subtask.drop (canon subtask.drop))
    (core func $task.return (canon task.return))
    (core module $m
      (import "libc" "mem" (memory 1))
      (import "" "slow" (func $slow (param i32) (result i32)))
      (import "" "waitable-set.new" (func $waitable-set.new (result i32)))
      (import "" "waitable-set.wait" (func $waitable-set.wait (param i32 i32) (result i32)))
      (import "" "waitable.join" (func $waitable.join (param i32 i32)))
      (import "" "subtask.drop" (func $subtask.drop (param i32)))
      (import "" "task.return" (func $task.return))
      (global $S (mut i32) (i32.const 0))

      ;; Start a slow host call and join it to $S.
      (func $start-slow (param $retp i32)
        (local $r i32)
        (local.set $r (call $slow (local.get $retp)))
        (if (i32.ne (i32.and (local.get $r) (i32.const 0xf))
              (i32.const 1 (; STARTED ;)))
          (then unreachable))
        (call $waitable.join (i32.shr_u (local.get $r) (i32.const 4)) (global.get $S)))

      (func (export "t1") (result i32)
        (global.set $S (call $waitable-set.new))
        (call $start-slow (i32.const 0))
        (i32.or (i32.const 2 (; WAIT ;))
          (i32.shl (global.get $S) (i32.const 4))))
      (func (export "t1-cb")
        (param $ev i32) (param $h i32) (param $st i32) (result i32)
        (if (i32.ne (local.get $ev) (i32.const 1 (; SUBTASK ;)))
          (then unreachable))
        (if (i32.ne (local.get $st) (i32.const 2 (; RETURNED ;)))
          (then unreachable))
        (call $subtask.drop (local.get $h))
        (call $task.return)
        (i32.const 0 (; EXIT ;)))

      (func (export "t2")
        (if (i32.ne (call $waitable-set.wait (global.get $S) (i32.const 8))
              (i32.const 1 (; SUBTASK ;)))
          (then unreachable))
        (if (i32.ne (i32.load (i32.const 12)) (i32.const 2 (; RETURNED ;)))
          (then unreachable))
        (call $subtask.drop (i32.load (i32.const 8)))
        ;; Give t1 something to wake up for.
        (call $start-slow (i32.const 16)))
    )
    (core instance $i (instantiate $m
      (with "libc" (instance $libc))
      (with "" (instance
        (export "slow" (func $slow))
        (export "waitable-set.new" (func $waitable-set.new))
        (export "waitable-set.wait" (func $waitable-set.wait))
        (export "waitable.join" (func $waitable.join))
        (export "subtask.drop" (func $subtask.drop))
        (export "task.return" (func $task.return))))))
    (func (export "t1") async
      (canon lift (core func $i "t1") async (callback (core func $i "t1-cb"))))
    (func (export "t2") async (canon lift (core func $i "t2"))))

  (component $C
    (import "t1" (func $t1 async))
    (import "t2" (func $t2 async))
    (core func $t1 (canon lower (func $t1) async))
    (core func $t2 (canon lower (func $t2) async))
    (core func $waitable-set.new (canon waitable-set.new))
    (core func $waitable.join (canon waitable.join))
    (core func $task.return (canon task.return (result u32)))
    (core module $m
      (import "" "t1" (func $t1 (result i32)))
      (import "" "t2" (func $t2 (result i32)))
      (import "" "waitable-set.new" (func $waitable-set.new (result i32)))
      (import "" "waitable.join" (func $waitable.join (param i32 i32)))
      (import "" "task.return" (func $task.return (param i32)))
      (global $set (mut i32) (i32.const 0))
      (global $done (mut i32) (i32.const 0))
      (func (export "run") (result i32)
        (local $a i32) (local $b i32)
        (local.set $a (call $t1))
        (if (i32.ne (i32.and (local.get $a) (i32.const 0xf))
              (i32.const 1 (; STARTED ;)))
          (then unreachable))
        (local.set $b (call $t2))
        (if (i32.ne (i32.and (local.get $b) (i32.const 0xf))
              (i32.const 1 (; STARTED ;)))
          (then unreachable))
        (global.set $set (call $waitable-set.new))
        (call $waitable.join (i32.shr_u (local.get $a) (i32.const 4)) (global.get $set))
        (call $waitable.join (i32.shr_u (local.get $b) (i32.const 4)) (global.get $set))
        (i32.or (i32.const 2 (; WAIT ;))
          (i32.shl (global.get $set) (i32.const 4))))
      (func (export "run-cb")
        (param $ev i32) (param $h i32) (param $st i32) (result i32)
        (if (i32.ne (local.get $ev) (i32.const 1 (; SUBTASK ;)))
          (then unreachable))
        (if (i32.eq (local.get $st) (i32.const 2 (; RETURNED ;)))
          (then (global.set $done (i32.add (global.get $done) (i32.const 1)))))
        (if (i32.eq (global.get $done) (i32.const 2))
          (then
            (call $task.return (i32.const 42))
            (return (i32.const 0 (; EXIT ;)))))
        (i32.or (i32.const 2 (; WAIT ;))
          (i32.shl (global.get $set) (i32.const 4)))))
    (core instance $i (instantiate $m (with "" (instance
      (export "t1" (func $t1))
      (export "t2" (func $t2))
      (export "waitable-set.new" (func $waitable-set.new))
      (export "waitable.join" (func $waitable.join))
      (export "task.return" (func $task.return))))))
    (func (export "run") async (result u32)
      (canon lift (core func $i "run") async
        (callback (core func $i "run-cb")))))

  (instance $x (instantiate $X (with "slow" (func $host "return-two-slowly"))))
  (instance $c (instantiate $C
    (with "t1" (func $x "t1"))
    (with "t2" (func $x "t2"))))
  (export "run" (func $c "run"))
)

(assert_return (invoke "run") (u32.const 42))

;; Same as above, but with a second callback-lifted task t1b also waiting on $S
;; (with no host call of its own) alongside t1 and t2. Neither t1 nor t1b can
;; run while t2 holds $X's exclusive lock, so t2 must still get the first
;; event. It then starts two more host calls joined to $S, one for each of t1
;; and t1b, and returns.
(component
  (import "host" (instance $host
    (export "return-two-slowly" (func async (result s32)))))

  (component $X
    (import "slow" (func $slow async (result s32)))
    (core module $libc (memory (export "mem") 1))
    (core instance $libc (instantiate $libc))
    (core func $slow
      (canon lower (func $slow) async (memory (core memory $libc "mem"))))
    (core func $waitable-set.new (canon waitable-set.new))
    (core func $waitable-set.wait
      (canon waitable-set.wait (memory (core memory $libc "mem"))))
    (core func $waitable.join (canon waitable.join))
    (core func $subtask.drop (canon subtask.drop))
    (core func $task.return (canon task.return))
    (core module $m
      (import "libc" "mem" (memory 1))
      (import "" "slow" (func $slow (param i32) (result i32)))
      (import "" "waitable-set.new" (func $waitable-set.new (result i32)))
      (import "" "waitable-set.wait" (func $waitable-set.wait (param i32 i32) (result i32)))
      (import "" "waitable.join" (func $waitable.join (param i32 i32)))
      (import "" "subtask.drop" (func $subtask.drop (param i32)))
      (import "" "task.return" (func $task.return))
      (global $S (mut i32) (i32.const 0))

      ;; Start a slow host call and join it to $S.
      (func $start-slow (param $retp i32)
        (local $r i32)
        (local.set $r (call $slow (local.get $retp)))
        (if (i32.ne (i32.and (local.get $r) (i32.const 0xf))
              (i32.const 1 (; STARTED ;)))
          (then unreachable))
        (call $waitable.join (i32.shr_u (local.get $r) (i32.const 4)) (global.get $S)))

      (func (export "t1") (result i32)
        (global.set $S (call $waitable-set.new))
        (call $start-slow (i32.const 0))
        (i32.or (i32.const 2 (; WAIT ;))
          (i32.shl (global.get $S) (i32.const 4))))
      (func (export "t1b") (result i32)
        (i32.or (i32.const 2 (; WAIT ;))
          (i32.shl (global.get $S) (i32.const 4))))
      (func (export "t1-cb")
        (param $ev i32) (param $h i32) (param $st i32) (result i32)
        (if (i32.ne (local.get $ev) (i32.const 1 (; SUBTASK ;)))
          (then unreachable))
        (if (i32.ne (local.get $st) (i32.const 2 (; RETURNED ;)))
          (then unreachable))
        (call $subtask.drop (local.get $h))
        (call $task.return)
        (i32.const 0 (; EXIT ;)))

      (func (export "t2")
        (if (i32.ne (call $waitable-set.wait (global.get $S) (i32.const 8))
              (i32.const 1 (; SUBTASK ;)))
          (then unreachable))
        (if (i32.ne (i32.load (i32.const 12)) (i32.const 2 (; RETURNED ;)))
          (then unreachable))
        (call $subtask.drop (i32.load (i32.const 8)))
        ;; Give t1 and t1b something to wake up for.
        (call $start-slow (i32.const 16))
        (call $start-slow (i32.const 24)))
    )
    (core instance $i (instantiate $m
      (with "libc" (instance $libc))
      (with "" (instance
        (export "slow" (func $slow))
        (export "waitable-set.new" (func $waitable-set.new))
        (export "waitable-set.wait" (func $waitable-set.wait))
        (export "waitable.join" (func $waitable.join))
        (export "subtask.drop" (func $subtask.drop))
        (export "task.return" (func $task.return))))))
    (func (export "t1") async
      (canon lift (core func $i "t1") async (callback (core func $i "t1-cb"))))
    (func (export "t1b") async
      (canon lift (core func $i "t1b") async (callback (core func $i "t1-cb"))))
    (func (export "t2") async (canon lift (core func $i "t2"))))

  (component $C
    (import "t1" (func $t1 async))
    (import "t1b" (func $t1b async))
    (import "t2" (func $t2 async))
    (core func $t1 (canon lower (func $t1) async))
    (core func $t1b (canon lower (func $t1b) async))
    (core func $t2 (canon lower (func $t2) async))
    (core func $waitable-set.new (canon waitable-set.new))
    (core func $waitable.join (canon waitable.join))
    (core func $task.return (canon task.return (result u32)))
    (core module $m
      (import "" "t1" (func $t1 (result i32)))
      (import "" "t1b" (func $t1b (result i32)))
      (import "" "t2" (func $t2 (result i32)))
      (import "" "waitable-set.new" (func $waitable-set.new (result i32)))
      (import "" "waitable.join" (func $waitable.join (param i32 i32)))
      (import "" "task.return" (func $task.return (param i32)))
      (global $set (mut i32) (i32.const 0))
      (global $done (mut i32) (i32.const 0))
      (func (export "run") (result i32)
        (local $a i32) (local $b i32) (local $c i32)
        (local.set $a (call $t1))
        (if (i32.ne (i32.and (local.get $a) (i32.const 0xf))
              (i32.const 1 (; STARTED ;)))
          (then unreachable))
        (local.set $c (call $t1b))
        (if (i32.ne (i32.and (local.get $c) (i32.const 0xf))
              (i32.const 1 (; STARTED ;)))
          (then unreachable))
        (local.set $b (call $t2))
        (if (i32.ne (i32.and (local.get $b) (i32.const 0xf))
              (i32.const 1 (; STARTED ;)))
          (then unreachable))
        (global.set $set (call $waitable-set.new))
        (call $waitable.join (i32.shr_u (local.get $a) (i32.const 4)) (global.get $set))
        (call $waitable.join (i32.shr_u (local.get $b) (i32.const 4)) (global.get $set))
        (call $waitable.join (i32.shr_u (local.get $c) (i32.const 4)) (global.get $set))
        (i32.or (i32.const 2 (; WAIT ;))
          (i32.shl (global.get $set) (i32.const 4))))
      (func (export "run-cb")
        (param $ev i32) (param $h i32) (param $st i32) (result i32)
        (if (i32.ne (local.get $ev) (i32.const 1 (; SUBTASK ;)))
          (then unreachable))
        (if (i32.eq (local.get $st) (i32.const 2 (; RETURNED ;)))
          (then (global.set $done (i32.add (global.get $done) (i32.const 1)))))
        (if (i32.eq (global.get $done) (i32.const 3))
          (then
            (call $task.return (i32.const 42))
            (return (i32.const 0 (; EXIT ;)))))
        (i32.or (i32.const 2 (; WAIT ;))
          (i32.shl (global.get $set) (i32.const 4)))))
    (core instance $i (instantiate $m (with "" (instance
      (export "t1" (func $t1))
      (export "t1b" (func $t1b))
      (export "t2" (func $t2))
      (export "waitable-set.new" (func $waitable-set.new))
      (export "waitable.join" (func $waitable.join))
      (export "task.return" (func $task.return))))))
    (func (export "run") async (result u32)
      (canon lift (core func $i "run") async
        (callback (core func $i "run-cb")))))

  (instance $x (instantiate $X (with "slow" (func $host "return-two-slowly"))))
  (instance $c (instantiate $C
    (with "t1" (func $x "t1"))
    (with "t1b" (func $x "t1b"))
    (with "t2" (func $x "t2"))))
  (export "run" (func $c "run"))
)

(assert_return (invoke "run") (u32.const 42))

;; As in the first case, t1 (callback-lifted, WAIT on $S) and t2 (sync-lifted,
;; async-typed, blocked in `waitable-set.wait` on $S) wait on $S, and t2 takes
;; the event from t1's host call A. Here t2 starts no further host call, so
;; once t2 returns and the exclusive lock is released, $S has no event for t1
;; and t1 keeps waiting. Only after t2 has returned does $C call t3, which
;; starts a host call joined to $S and returns; t1's callback must see that
;; call's completion as its first and only event.
(component
  (import "host" (instance $host
    (export "return-two-slowly" (func async (result s32)))))

  (component $X
    (import "slow" (func $slow async (result s32)))
    (core module $libc (memory (export "mem") 1))
    (core instance $libc (instantiate $libc))
    (core func $slow
      (canon lower (func $slow) async (memory (core memory $libc "mem"))))
    (core func $waitable-set.new (canon waitable-set.new))
    (core func $waitable-set.wait
      (canon waitable-set.wait (memory (core memory $libc "mem"))))
    (core func $waitable.join (canon waitable.join))
    (core func $subtask.drop (canon subtask.drop))
    (core func $task.return (canon task.return))
    (core module $m
      (import "libc" "mem" (memory 1))
      (import "" "slow" (func $slow (param i32) (result i32)))
      (import "" "waitable-set.new" (func $waitable-set.new (result i32)))
      (import "" "waitable-set.wait" (func $waitable-set.wait (param i32 i32) (result i32)))
      (import "" "waitable.join" (func $waitable.join (param i32 i32)))
      (import "" "subtask.drop" (func $subtask.drop (param i32)))
      (import "" "task.return" (func $task.return))
      (global $S (mut i32) (i32.const 0))
      (global $late (mut i32) (i32.const 0))

      (func $start-slow (param $retp i32)
        (local $r i32)
        (local.set $r (call $slow (local.get $retp)))
        (if (i32.ne (i32.and (local.get $r) (i32.const 0xf))
              (i32.const 1 (; STARTED ;)))
          (then unreachable))
        (call $waitable.join (i32.shr_u (local.get $r) (i32.const 4)) (global.get $S)))

      (func (export "t1") (result i32)
        (global.set $S (call $waitable-set.new))
        (call $start-slow (i32.const 0))
        (i32.or (i32.const 2 (; WAIT ;))
          (i32.shl (global.get $S) (i32.const 4))))
      (func (export "t1-cb")
        (param $ev i32) (param $h i32) (param $st i32) (result i32)
        (if (i32.ne (local.get $ev) (i32.const 1 (; SUBTASK ;)))
          (then unreachable))
        (if (i32.ne (local.get $h) (global.get $late))
          (then unreachable))
        (if (i32.ne (local.get $st) (i32.const 2 (; RETURNED ;)))
          (then unreachable))
        (call $subtask.drop (local.get $h))
        (call $task.return)
        (i32.const 0 (; EXIT ;)))

      (func (export "t2")
        (if (i32.ne (call $waitable-set.wait (global.get $S) (i32.const 8))
              (i32.const 1 (; SUBTASK ;)))
          (then unreachable))
        (if (i32.ne (i32.load (i32.const 12)) (i32.const 2 (; RETURNED ;)))
          (then unreachable))
        (call $subtask.drop (i32.load (i32.const 8))))

      (func (export "t3") (result i32)
        (local $r i32)
        (local.set $r (call $slow (i32.const 48)))
        (if (i32.ne (i32.and (local.get $r) (i32.const 0xf))
              (i32.const 1 (; STARTED ;)))
          (then unreachable))
        (global.set $late (i32.shr_u (local.get $r) (i32.const 4)))
        (call $waitable.join (global.get $late) (global.get $S))
        (call $task.return)
        (i32.const 0 (; EXIT ;)))
      (func (export "t3-cb")
        (param i32 i32 i32) (result i32)
        unreachable)
    )
    (core instance $i (instantiate $m
      (with "libc" (instance $libc))
      (with "" (instance
        (export "slow" (func $slow))
        (export "waitable-set.new" (func $waitable-set.new))
        (export "waitable-set.wait" (func $waitable-set.wait))
        (export "waitable.join" (func $waitable.join))
        (export "subtask.drop" (func $subtask.drop))
        (export "task.return" (func $task.return))))))
    (func (export "t1") async
      (canon lift (core func $i "t1") async (callback (core func $i "t1-cb"))))
    (func (export "t2") async (canon lift (core func $i "t2")))
    (func (export "t3") async
      (canon lift (core func $i "t3") async (callback (core func $i "t3-cb")))))

  (component $C
    (import "t1" (func $t1 async))
    (import "t2" (func $t2 async))
    (import "t3" (func $t3 async))
    (core func $t1 (canon lower (func $t1) async))
    (core func $t2 (canon lower (func $t2) async))
    (core func $t3 (canon lower (func $t3) async))
    (core func $waitable-set.new (canon waitable-set.new))
    (core func $waitable.join (canon waitable.join))
    (core func $task.return (canon task.return (result u32)))
    (core module $m
      (import "" "t1" (func $t1 (result i32)))
      (import "" "t2" (func $t2 (result i32)))
      (import "" "t3" (func $t3 (result i32)))
      (import "" "waitable-set.new" (func $waitable-set.new (result i32)))
      (import "" "waitable.join" (func $waitable.join (param i32 i32)))
      (import "" "task.return" (func $task.return (param i32)))
      (global $set (mut i32) (i32.const 0))
      (global $done (mut i32) (i32.const 0))
      (global $t2 (mut i32) (i32.const 0))
      (func $call (param $r i32)
        (if (i32.ne (i32.and (local.get $r) (i32.const 0xf))
              (i32.const 1 (; STARTED ;)))
          (then unreachable))
        (call $waitable.join (i32.shr_u (local.get $r) (i32.const 4)) (global.get $set)))
      (func (export "run") (result i32)
        (local $a i32)
        (global.set $set (call $waitable-set.new))
        (call $call (call $t1))
        (local.set $a (call $t2))
        (global.set $t2 (i32.shr_u (local.get $a) (i32.const 4)))
        (call $call (local.get $a))
        (i32.or (i32.const 2 (; WAIT ;))
          (i32.shl (global.get $set) (i32.const 4))))
      (func (export "run-cb")
        (param $ev i32) (param $h i32) (param $st i32) (result i32)
        (local $r i32)
        (if (i32.ne (local.get $ev) (i32.const 1 (; SUBTASK ;)))
          (then unreachable))
        (if (i32.eq (local.get $st) (i32.const 2 (; RETURNED ;)))
          (then
            (global.set $done (i32.add (global.get $done) (i32.const 1)))
            ;; Only once t2 has returned (and so t1 has been unparked and
            ;; found nothing) do we produce the event t1 is waiting for.
            (if (i32.eq (local.get $h) (global.get $t2))
              (then
                (local.set $r (call $t3))
                (if (i32.eq (i32.and (local.get $r) (i32.const 0xf))
                      (i32.const 2 (; RETURNED ;)))
                  (then (global.set $done
                    (i32.add (global.get $done) (i32.const 1))))
                  (else (call $call (local.get $r))))))))
        (if (i32.eq (global.get $done) (i32.const 3))
          (then
            (call $task.return (i32.const 42))
            (return (i32.const 0 (; EXIT ;)))))
        (i32.or (i32.const 2 (; WAIT ;))
          (i32.shl (global.get $set) (i32.const 4)))))
    (core instance $i (instantiate $m (with "" (instance
      (export "t1" (func $t1))
      (export "t2" (func $t2))
      (export "t3" (func $t3))
      (export "waitable-set.new" (func $waitable-set.new))
      (export "waitable.join" (func $waitable.join))
      (export "task.return" (func $task.return))))))
    (func (export "run") async (result u32)
      (canon lift (core func $i "run") async
        (callback (core func $i "run-cb")))))

  (instance $x (instantiate $X (with "slow" (func $host "return-two-slowly"))))
  (instance $c (instantiate $C
    (with "t1" (func $x "t1"))
    (with "t2" (func $x "t2"))
    (with "t3" (func $x "t3"))))
  (export "run" (func $c "run"))
)

(assert_return (invoke "run") (u32.const 42))
