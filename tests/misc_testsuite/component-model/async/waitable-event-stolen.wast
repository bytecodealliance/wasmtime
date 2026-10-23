;;! component_model_async = true

;; Two callback-lifted tasks, "a" and "b", in the same instance share a
;; waitable set. "a" waits on the set (returning `WAIT`), "b" completes a read
;; that's in the set (waking "a" up), and then, before "a" gets to run, "b"
;; takes the event itself or moves the waitable out of the set. Here "a" should
;; keep waiting on the set.
;;
;; The `mode` parameter to `run` selects what "b" does:
;;
;; * 0: takes the event with `waitable-set.poll`
;; * 1: takes the event with `waitable-set.poll`, then completes a second read
;;   in the set, which "a" receives
;; * 2: removes the waitable from the set with `waitable.join(w, 0)`
;; * 3: moves the waitable to another set and polls that set
;; * 4: removes all waitables from the set and tries to drop it, which traps
;;   since "a" is still waiting on it
;;
;; If bit 3 (8) of `mode` is set, "a" completes the first read itself before
;; returning `WAIT`, so "a" is scheduled to receive the event immediately
;; without first registering as a waiter.
(component definition $T
  (component $J
    (core module $libc (memory (export "m") 1))
    (core instance $libc (instantiate $libc))
    (type $f (future))
    (core func $future.new (canon future.new $f))
    (core func $future.read (canon future.read $f async))
    (core func $future.write (canon future.write $f async))
    (core func $waitable-set.new (canon waitable-set.new))
    (core func $waitable-set.drop (canon waitable-set.drop))
    (core func $waitable.join (canon waitable.join))
    (core func $waitable-set.poll
      (canon waitable-set.poll (memory (core memory $libc "m"))))
    (core func $task.return (canon task.return))
    (core module $m
      (import "" "m" (memory 1))
      (import "" "future.new" (func $future.new (result i64)))
      (import "" "future.read" (func $future.read (param i32 i32) (result i32)))
      (import "" "future.write"
        (func $future.write (param i32 i32) (result i32)))
      (import "" "waitable-set.new" (func $ws.new (result i32)))
      (import "" "waitable-set.drop" (func $ws.drop (param i32)))
      (import "" "waitable.join" (func $join (param i32 i32)))
      (import "" "waitable-set.poll" (func $poll (param i32 i32) (result i32)))
      (import "" "task.return" (func $ret))
      (global $set (mut i32) (i32.const 0))
      (global $r1 (mut i32) (i32.const 0))
      (global $w1 (mut i32) (i32.const 0))
      (global $r2 (mut i32) (i32.const 0))
      (global $w2 (mut i32) (i32.const 0))

      (func (export "a") (param $mode i32) (result i32)
        (local $tmp i64)
        (local.set $tmp (call $future.new))
        (global.set $r1 (i32.wrap_i64 (local.get $tmp)))
        (global.set $w1
          (i32.wrap_i64 (i64.shr_u (local.get $tmp) (i64.const 32))))
        (local.set $tmp (call $future.new))
        (global.set $r2 (i32.wrap_i64 (local.get $tmp)))
        (global.set $w2
          (i32.wrap_i64 (i64.shr_u (local.get $tmp) (i64.const 32))))
        (if (i32.ne (call $future.read (global.get $r1) (i32.const 0))
              (i32.const -1 (; BLOCKED ;)))
          (then unreachable))
        (if (i32.ne (call $future.read (global.get $r2) (i32.const 0))
              (i32.const -1 (; BLOCKED ;)))
          (then unreachable))
        (global.set $set (call $ws.new))
        (call $join (global.get $r1) (global.get $set))
        (call $join (global.get $r2) (global.get $set))
        (if (i32.and (local.get $mode) (i32.const 8))
          (then
            (if (i32.ne (call $future.write (global.get $w1) (i32.const 0))
                  (i32.const 0 (; COMPLETED ;)))
              (then unreachable))))
        ;; CALLBACK_CODE_WAIT | (set << 4)
        (i32.or (i32.const 2) (i32.shl (global.get $set) (i32.const 4))))
      (func (export "a-cb")
        (param $event i32) (param $handle i32) (param i32) (result i32)
        ;; the only event "a" may receive is the completion of the second read
        (if (i32.ne (local.get $event) (i32.const 4 (; FUTURE_READ ;)))
          (then unreachable))
        (if (i32.ne (local.get $handle) (global.get $r2))
          (then unreachable))
        (call $ret)
        (i32.const 0 (; EXIT ;)))

      (func (export "b") (param $mode i32) (result i32)
        (local $set2 i32)
        (if (i32.eqz (i32.and (local.get $mode) (i32.const 8)))
          (then
            (if (i32.ne (call $future.write (global.get $w1) (i32.const 0))
                  (i32.const 0 (; COMPLETED ;)))
              (then unreachable))))
        (block $done
          (block $drop
            (block $join-other
              (block $join-none
                (block $poll-then-wake
                  (block $poll
                    (br_table $poll $poll-then-wake $join-none $join-other $drop
                      (i32.and (local.get $mode) (i32.const 7))))
                  ;; mode 0
                  (if (i32.ne (call $poll (global.get $set) (i32.const 0))
                        (i32.const 4 (; FUTURE_READ ;)))
                    (then unreachable))
                  (if (i32.ne (i32.load (i32.const 0)) (global.get $r1))
                    (then unreachable))
                  (br $done))
                ;; mode 1
                (if (i32.ne (call $poll (global.get $set) (i32.const 0))
                      (i32.const 4 (; FUTURE_READ ;)))
                  (then unreachable))
                (if (i32.ne (i32.load (i32.const 0)) (global.get $r1))
                  (then unreachable))
                (if (i32.ne (call $future.write (global.get $w2) (i32.const 0))
                      (i32.const 0 (; COMPLETED ;)))
                  (then unreachable))
                (br $done))
              ;; mode 2
              (call $join (global.get $r1) (i32.const 0))
              (br $done))
            ;; mode 3
            (local.set $set2 (call $ws.new))
            (call $join (global.get $r1) (local.get $set2))
            (if (i32.ne (call $poll (local.get $set2) (i32.const 0))
                  (i32.const 4 (; FUTURE_READ ;)))
              (then unreachable))
            (br $done))
          ;; mode 4
          (call $join (global.get $r1) (i32.const 0))
          (call $join (global.get $r2) (i32.const 0))
          (call $ws.drop (global.get $set)))
        (call $ret)
        (i32.const 0 (; EXIT ;)))
      (func (export "b-cb") (param i32 i32 i32) (result i32) unreachable)
    )
    (core instance $i (instantiate $m (with "" (instance
      (export "m" (memory $libc "m"))
      (export "future.new" (func $future.new))
      (export "future.read" (func $future.read))
      (export "future.write" (func $future.write))
      (export "waitable-set.new" (func $waitable-set.new))
      (export "waitable-set.drop" (func $waitable-set.drop))
      (export "waitable.join" (func $waitable.join))
      (export "waitable-set.poll" (func $waitable-set.poll))
      (export "task.return" (func $task.return))))))
    (func (export "a") async (param "mode" u32)
      (canon lift (core func $i "a") async (callback (core func $i "a-cb"))))
    (func (export "b") async (param "mode" u32)
      (canon lift (core func $i "b") async (callback (core func $i "b-cb"))))
  )
  (component $C
    (import "a" (func $a async (param "mode" u32)))
    (import "b" (func $b async (param "mode" u32)))
    (core func $a (canon lower (func $a) async))
    (core func $b (canon lower (func $b) async))
    (core func $waitable-set.new (canon waitable-set.new))
    (core func $waitable.join (canon waitable.join))
    (core func $subtask.drop (canon subtask.drop))
    (core func $task.return (canon task.return (result u32)))
    (core module $m
      (import "" "a" (func $a (param i32) (result i32)))
      (import "" "b" (func $b (param i32) (result i32)))
      (import "" "waitable-set.new" (func $ws.new (result i32)))
      (import "" "waitable.join" (func $join (param i32 i32)))
      (import "" "subtask.drop" (func $subtask.drop (param i32)))
      (import "" "task.return" (func $task.return (param i32)))
      (global $sa (mut i32) (i32.const 0))
      (func (export "run") (param $mode i32) (result i32)
        (local $set i32)
        ;; start "a"; it blocks, so we get a subtask handle (STARTED=1)
        (global.set $sa (call $a (local.get $mode)))
        (if (i32.ne (i32.and (global.get $sa) (i32.const 0xf)) (i32.const 1))
          (then unreachable))
        (global.set $sa (i32.shr_u (global.get $sa) (i32.const 4)))
        ;; "b" runs to completion (RETURNED=2)
        (if (i32.ne (call $b (local.get $mode)) (i32.const 2 (; RETURNED ;)))
          (then unreachable))
        ;; now wait for "a"
        (local.set $set (call $ws.new))
        (call $join (global.get $sa) (local.get $set))
        ;; CALLBACK_CODE_WAIT | (set << 4)
        (i32.or (i32.const 2) (i32.shl (local.get $set) (i32.const 4))))
      (func (export "run-cb")
        (param $event i32) (param $handle i32) (param $status i32)
        (result i32)
        (if (i32.ne (local.get $event) (i32.const 1 (; SUBTASK ;)))
          (then unreachable))
        (if (i32.ne (local.get $handle) (global.get $sa))
          (then unreachable))
        (if (i32.ne (local.get $status) (i32.const 2 (; RETURNED ;)))
          (then unreachable))
        (call $subtask.drop (global.get $sa))
        (call $task.return (i32.const 7))
        (i32.const 0 (; EXIT ;)))
    )
    (core instance $i (instantiate $m (with "" (instance
      (export "a" (func $a))
      (export "b" (func $b))
      (export "waitable-set.new" (func $waitable-set.new))
      (export "waitable.join" (func $waitable.join))
      (export "subtask.drop" (func $subtask.drop))
      (export "task.return" (func $task.return))))))
    (func (export "run") async (param "mode" u32) (result u32)
      (canon lift (core func $i "run") async
        (callback (core func $i "run-cb"))))
  )
  (instance $j (instantiate $J))
  (instance $c (instantiate $C
    (with "a" (func $j "a"))
    (with "b" (func $j "b"))))
  (func (export "run") (alias export $c "run"))
)

(component instance $T $T)
(assert_trap (invoke "run" (u32.const 0)) "deadlock detected")
(component instance $T $T)
(assert_return (invoke "run" (u32.const 1)) (u32.const 7))
(component instance $T $T)
(assert_trap (invoke "run" (u32.const 2)) "deadlock detected")
(component instance $T $T)
(assert_trap (invoke "run" (u32.const 3)) "deadlock detected")
(component instance $T $T)
(assert_trap (invoke "run" (u32.const 4))
  "cannot drop waitable set with waiters")

(component instance $T $T)
(assert_trap (invoke "run" (u32.const 8)) "deadlock detected")
(component instance $T $T)
(assert_return (invoke "run" (u32.const 9)) (u32.const 7))
(component instance $T $T)
(assert_trap (invoke "run" (u32.const 10)) "deadlock detected")
(component instance $T $T)
(assert_trap (invoke "run" (u32.const 11)) "deadlock detected")
(component instance $T $T)
(assert_trap (invoke "run" (u32.const 12))
  "cannot drop waitable set with waiters")
