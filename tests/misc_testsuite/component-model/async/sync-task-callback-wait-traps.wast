;;! component_model_async = true

;; A sync-typed task `s` in instance $A blocks on a slow (async) host import.
;; Here the only other thread of $A is the implicit thread of callback-lifted
;; task `t`, which is ready (it returned YIELD). It is resumed, and its
;; callback then returns WAIT on an empty waitable set, so it blocks. No thread
;; of $A is ready anymore, so this must trap.
(component
  (import "host" (instance $host
    (export "return-two-slowly" (func async (result s32)))
  ))

  (component $A
    (import "host" (instance $host
      (export "return-two-slowly" (func async (result s32)))
    ))
    (core func $slow (canon lower (func $host "return-two-slowly")))
    (core func $task.cancel (canon task.cancel))
    (core func $waitable-set.new (canon waitable-set.new))
    (core module $m
      (import "" "slow" (func $slow (result i32)))
      (import "" "task.cancel" (func $task.cancel))
      (import "" "waitable-set.new" (func $waitable-set.new (result i32)))
      (global $ws (mut i32) (i32.const 0))

      ;; callback-lifted: first YIELD, then WAIT on an empty set until
      ;; cancelled.
      (func (export "t") (result i32) (i32.const 1 (; YIELD ;)))
      (func (export "t-cb") (param $ev i32) (param i32 i32) (result i32)
        (if (i32.eq (local.get $ev) (i32.const 6 (; TASK_CANCELLED ;)))
          (then (call $task.cancel) (return (i32.const 0 (; EXIT ;)))))
        (if (i32.eqz (global.get $ws))
          (then (global.set $ws (call $waitable-set.new))))
        (i32.or (i32.const 2 (; WAIT ;))
          (i32.shl (global.get $ws) (i32.const 4))))

      ;; sync-typed, sync-lifted: blocks on the slow host import
      (func (export "s") (result i32) (call $slow))
    )
    (core instance $i (instantiate $m
      (with "" (instance
        (export "slow" (func $slow))
        (export "task.cancel" (func $task.cancel))
        (export "waitable-set.new" (func $waitable-set.new))
      ))
    ))
    (func (export "t") async
      (canon lift (core func $i "t") async (callback (core func $i "t-cb"))))
    (func (export "s") (result s32) (canon lift (core func $i "s")))
  )
  (instance $a (instantiate $A (with "host" (instance $host))))

  (component $B
    (import "a" (instance $a
      (export "t" (func async))
      (export "s" (func (result s32)))
    ))
    (core func $t (canon lower (func $a "t") async))
    (core func $s (canon lower (func $a "s")))
    (core func $subtask.cancel (canon subtask.cancel))
    (core func $subtask.drop (canon subtask.drop))
    (core module $m
      (import "" "t" (func $t (result i32)))
      (import "" "s" (func $s (result i32)))
      (import "" "subtask.cancel"
        (func $subtask.cancel (param i32) (result i32)))
      (import "" "subtask.drop" (func $subtask.drop (param i32)))
      (func (export "run") (result i32)
        (local $rc i32)
        ;; start `t`; it yields, so it's STARTED
        (local.set $rc (call $t))
        (if (i32.ne (i32.and (local.get $rc) (i32.const 0xf))
              (i32.const 1 (; STARTED ;)))
          (then unreachable))

        ;; this should trap
        call $s

        ;; .. so shouldn't get there
        unreachable
      )
    )
    (core instance $i (instantiate $m
      (with "" (instance
        (export "t" (func $t))
        (export "s" (func $s))
        (export "subtask.cancel" (func $subtask.cancel))
        (export "subtask.drop" (func $subtask.drop))
      ))
    ))
    (func (export "run") async (result s32) (canon lift (core func $i "run")))
  )
  (instance $b (instantiate $B (with "a" (instance $a))))
  (export "run" (func $b "run"))
)

(assert_trap (invoke "run") "cannot block a synchronous task")


;; Cross-instance variant: sync-typed `$A.s` sync-calls `$C.w`, which
;; itself waits on the slow async host import. While `s` is blocked the
;; only other thread of `$A` is callback task `t`, which returns WAIT on an
;; empty set and so is not ready. This should trap.
(component
  (import "host" (instance $host
    (export "return-two-slowly" (func async (result s32)))))

  ;; $C: async-typed callback-lifted export `w` which waits for the async
  ;; host import `slow` and then returns its result.
  (component $C
    (import "slow" (func $slow async (result s32)))
    (core module $libc (memory (export "mem") 1))
    (core instance $libc (instantiate $libc))
    (core func $slow
      (canon lower (func $slow) async (memory (core memory $libc "mem"))))
    (core func $task.return (canon task.return (result u32)))
    (core func $waitable-set.new (canon waitable-set.new))
    (core func $waitable.join (canon waitable.join))
    (core func $subtask.drop (canon subtask.drop))
    (core module $m
      (import "libc" "mem" (memory 1))
      (import "" "slow" (func $slow (param i32) (result i32)))
      (import "" "task.return" (func $task.return (param i32)))
      (import "" "waitable-set.new" (func $waitable-set.new (result i32)))
      (import "" "waitable.join" (func $waitable.join (param i32 i32)))
      (import "" "subtask.drop" (func $subtask.drop (param i32)))
      (func (export "w") (result i32)
        (local $rc i32) (local $s i32)
        (local.set $rc (call $slow (i32.const 100)))
        (if (i32.ne (i32.and (local.get $rc) (i32.const 0xf))
              (i32.const 1 (; STARTED ;)))
          (then unreachable))
        (local.set $s (call $waitable-set.new))
        (call $waitable.join
          (i32.shr_u (local.get $rc) (i32.const 4)) (local.get $s))
        (i32.or (i32.const 2 (; WAIT ;))
          (i32.shl (local.get $s) (i32.const 4))))
      (func (export "w-cb")
        (param $ev i32) (param $h i32) (param $st i32) (result i32)
        (if (i32.ne (local.get $ev) (i32.const 1 (; SUBTASK ;)))
          (then unreachable))
        (if (i32.ne (local.get $st) (i32.const 2 (; RETURNED ;)))
          (then unreachable))
        (call $waitable.join (local.get $h) (i32.const 0))
        (call $subtask.drop (local.get $h))
        (call $task.return (i32.load (i32.const 100)))
        (i32.const 0 (; EXIT ;)))
    )
    (core instance $i (instantiate $m
      (with "libc" (instance $libc))
      (with "" (instance
        (export "slow" (func $slow))
        (export "task.return" (func $task.return))
        (export "waitable-set.new" (func $waitable-set.new))
        (export "waitable.join" (func $waitable.join))
        (export "subtask.drop" (func $subtask.drop))
      ))
    ))
    (func (export "w") async (result u32)
      (canon lift (core func $i "w") async (callback (core func $i "w-cb"))))
  )
  (instance $c (instantiate $C
    (with "slow" (func $host "return-two-slowly"))))

  ;; $A: callback-lifted `t` (YIELD, then WAIT on an empty set until
  ;; cancelled) and sync-typed `s` which sync-calls $C's `w`.
  (component $A
    (import "w" (func $w async (result u32)))
    (core func $w (canon lower (func $w)))
    (core func $task.cancel (canon task.cancel))
    (core func $waitable-set.new (canon waitable-set.new))
    (core module $m
      (import "" "w" (func $w (result i32)))
      (import "" "task.cancel" (func $task.cancel))
      (import "" "waitable-set.new" (func $waitable-set.new (result i32)))
      (global $ws (mut i32) (i32.const 0))
      (func (export "t") (result i32) (i32.const 1 (; YIELD ;)))
      (func (export "t-cb") (param $ev i32) (param i32 i32) (result i32)
        (if (i32.eq (local.get $ev) (i32.const 6 (; TASK_CANCELLED ;)))
          (then (call $task.cancel) (return (i32.const 0 (; EXIT ;)))))
        (if (i32.eqz (global.get $ws))
          (then (global.set $ws (call $waitable-set.new))))
        (i32.or (i32.const 2 (; WAIT ;))
          (i32.shl (global.get $ws) (i32.const 4))))
      (func (export "s") (result i32) (call $w))
    )
    (core instance $i (instantiate $m
      (with "" (instance
        (export "w" (func $w))
        (export "task.cancel" (func $task.cancel))
        (export "waitable-set.new" (func $waitable-set.new))
      ))
    ))
    (func (export "t") async
      (canon lift (core func $i "t") async (callback (core func $i "t-cb"))))
    (func (export "s") (result u32) (canon lift (core func $i "s")))
  )
  (instance $a (instantiate $A (with "w" (func $c "w"))))

  (component $B
    (import "t" (func $t async))
    (import "s" (func $s (result u32)))
    (core func $t (canon lower (func $t) async))
    (core func $s (canon lower (func $s)))
    (core func $subtask.cancel (canon subtask.cancel))
    (core func $subtask.drop (canon subtask.drop))
    (core module $m
      (import "" "t" (func $t (result i32)))
      (import "" "s" (func $s (result i32)))
      (import "" "subtask.cancel"
        (func $subtask.cancel (param i32) (result i32)))
      (import "" "subtask.drop" (func $subtask.drop (param i32)))
      (func (export "run") (result i32)
        (local $rc i32) (local $sub i32) (local $r i32)
        (local.set $rc (call $t))
        (if (i32.ne (i32.and (local.get $rc) (i32.const 0xf))
              (i32.const 1 (; STARTED ;)))
          (then unreachable))
        (local.set $sub (i32.shr_u (local.get $rc) (i32.const 4)))
        (local.set $r (call $s))
        (drop (call $subtask.cancel (local.get $sub)))
        (call $subtask.drop (local.get $sub))
        (local.get $r))
    )
    (core instance $i (instantiate $m
      (with "" (instance
        (export "t" (func $t))
        (export "s" (func $s))
        (export "subtask.cancel" (func $subtask.cancel))
        (export "subtask.drop" (func $subtask.drop))
      ))
    ))
    (func (export "run") async (result u32) (canon lift (core func $i "run")))
  )
  (instance $b (instantiate $B
    (with "t" (func $a "t"))
    (with "s" (func $a "s"))))
  (export "run" (func $b "run"))
)
(assert_trap (invoke "run") "cannot block a synchronous task")
