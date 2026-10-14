;;! component_model_async = true

;; Per the spec's `canon_subtask_cancel`, a synchronous `subtask.cancel` whose
;; subtask doesn't resolve right away blocks with `Thread.wait_until`, like any
;; other blocking built-in, so control returns to the thread that resumed the
;; cancelling task, here its caller.

;; `$C.run` async-calls `$B.g`, which async-calls `$A.f` and then synchronously
;; cancels it. `$A.f` ignores TASK_CANCELLED and keeps waiting, so the cancel
;; blocks and `$C.run`'s call of `$B.g` returns STARTED.
(component
  (component $A
    (core func $waitable-set.new (canon waitable-set.new))
    (core module $m
      (import "" "waitable-set.new" (func $waitable-set.new (result i32)))
      (global $set (mut i32) (i32.const 0))
      ;; WAIT on an empty set forever, ignoring TASK_CANCELLED
      (func (export "f") (result i32)
        (global.set $set (call $waitable-set.new))
        (i32.or (i32.shl (global.get $set) (i32.const 4)) (i32.const 2)))
      (func (export "f-cb") (param i32 i32 i32) (result i32)
        (i32.or (i32.shl (global.get $set) (i32.const 4)) (i32.const 2))))
    (core instance $i (instantiate $m (with "" (instance
      (export "waitable-set.new" (func $waitable-set.new))))))
    (func (export "f") async
      (canon lift (core func $i "f") async (callback (core func $i "f-cb")))))

  (component $B
    (import "f" (func $f async))
    (core func $f (canon lower (func $f) async))
    (core func $subtask.cancel (canon subtask.cancel))
    (core module $m
      (import "" "f" (func $f (result i32)))
      (import "" "subtask.cancel" (func $subtask.cancel (param i32) (result i32)))
      (func (export "g") (result i32)
        ;; `$A.f` blocks: subtask 1, STARTED
        (if (i32.ne (call $f) (i32.const 0x11)) (then unreachable))
        ;; `$A.f` never resolves, so this blocks forever
        (drop (call $subtask.cancel (i32.const 1)))
        unreachable)
      (func (export "g-cb") (param i32 i32 i32) (result i32) unreachable))
    (core instance $i (instantiate $m (with "" (instance
      (export "f" (func $f))
      (export "subtask.cancel" (func $subtask.cancel))))))
    (func (export "g") async
      (canon lift (core func $i "g") async (callback (core func $i "g-cb")))))

  (component $C
    (import "g" (func $g async))
    (core func $g (canon lower (func $g) async))
    (core func $task.return (canon task.return))
    (core module $m
      (import "" "g" (func $g (result i32)))
      (import "" "task.return" (func $task.return))
      (func (export "run") (result i32)
        ;; `$B.g` blocks in `subtask.cancel`: subtask 1, STARTED
        (if (i32.ne (call $g) (i32.const 0x11)) (then unreachable))
        (call $task.return)
        (i32.const 0 (; EXIT ;)))
      (func (export "run-cb") (param i32 i32 i32) (result i32) unreachable))
    (core instance $i (instantiate $m (with "" (instance
      (export "g" (func $g))
      (export "task.return" (func $task.return))))))
    (func (export "run") async
      (canon lift (core func $i "run") async (callback (core func $i "run-cb")))))

  (instance $a (instantiate $A))
  (instance $b (instantiate $B (with "f" (func $a "f"))))
  (instance $c (instantiate $C (with "g" (func $b "g"))))
  (export "run" (func $c "run")))

(assert_return (invoke "run"))

;; Same as above, but `$B.g` returns its value with `task.return` before it
;; cancels `$A.f`, and `$C.run` calls `$B.g` with a synchronous lower. Once
;; `$B.g` blocks in `subtask.cancel`, `$C.run`'s call has its result and
;; returns.
(component
  (component $A
    (core func $waitable-set.new (canon waitable-set.new))
    (core module $m
      (import "" "waitable-set.new" (func $waitable-set.new (result i32)))
      (global $set (mut i32) (i32.const 0))
      ;; WAIT on an empty set forever, ignoring TASK_CANCELLED
      (func (export "f") (result i32)
        (global.set $set (call $waitable-set.new))
        (i32.or (i32.shl (global.get $set) (i32.const 4)) (i32.const 2)))
      (func (export "f-cb") (param i32 i32 i32) (result i32)
        (i32.or (i32.shl (global.get $set) (i32.const 4)) (i32.const 2))))
    (core instance $i (instantiate $m (with "" (instance
      (export "waitable-set.new" (func $waitable-set.new))))))
    (func (export "f") async
      (canon lift (core func $i "f") async (callback (core func $i "f-cb")))))

  (component $B
    (import "f" (func $f async))
    (core func $f (canon lower (func $f) async))
    (core func $subtask.cancel (canon subtask.cancel))
    (core func $task.return (canon task.return (result u32)))
    (core module $m
      (import "" "f" (func $f (result i32)))
      (import "" "subtask.cancel" (func $subtask.cancel (param i32) (result i32)))
      (import "" "task.return" (func $task.return (param i32)))
      (func (export "g") (result i32)
        (call $task.return (i32.const 42))
        ;; `$A.f` blocks: subtask 1, STARTED
        (if (i32.ne (call $f) (i32.const 0x11)) (then unreachable))
        ;; `$A.f` never resolves, so this blocks forever
        (drop (call $subtask.cancel (i32.const 1)))
        unreachable)
      (func (export "g-cb") (param i32 i32 i32) (result i32) unreachable))
    (core instance $i (instantiate $m (with "" (instance
      (export "f" (func $f))
      (export "subtask.cancel" (func $subtask.cancel))
      (export "task.return" (func $task.return))))))
    (func (export "g") async (result u32)
      (canon lift (core func $i "g") async (callback (core func $i "g-cb")))))

  (component $C
    (import "g" (func $g async (result u32)))
    (core func $g (canon lower (func $g)))
    (core func $task.return (canon task.return (result u32)))
    (core module $m
      (import "" "g" (func $g (result i32)))
      (import "" "task.return" (func $task.return (param i32)))
      (func (export "run") (result i32)
        (call $task.return (call $g))
        (i32.const 0 (; EXIT ;)))
      (func (export "run-cb") (param i32 i32 i32) (result i32) unreachable))
    (core instance $i (instantiate $m (with "" (instance
      (export "g" (func $g))
      (export "task.return" (func $task.return))))))
    (func (export "run") async (result u32)
      (canon lift (core func $i "run") async (callback (core func $i "run-cb")))))

  (instance $a (instantiate $A))
  (instance $b (instantiate $B (with "f" (func $a "f"))))
  (instance $c (instantiate $C (with "g" (func $b "g"))))
  (export "run" (func $c "run")))

(assert_return (invoke "run") (u32.const 42))

;; A nested cancel: `$D.run` async-calls `$C.g`, which async-calls `$B.f` and
;; then synchronously cancels it. `$B.f`, waiting on its own subtask `$A.f`,
;; receives TASK_CANCELLED and in turn synchronously cancels `$A.f`, which
;; ignores TASK_CANCELLED and keeps waiting. `$B.f`'s cancel blocks, so control
;; returns to `$C.g`, whose cancel of `$B.f` also blocks, so `$D.run`'s call of
;; `$C.g` returns STARTED.
(component
  (component $A
    (core func $waitable-set.new (canon waitable-set.new))
    (core module $m
      (import "" "waitable-set.new" (func $waitable-set.new (result i32)))
      (global $set (mut i32) (i32.const 0))
      ;; WAIT on an empty set forever, ignoring TASK_CANCELLED
      (func (export "f") (result i32)
        (global.set $set (call $waitable-set.new))
        (i32.or (i32.shl (global.get $set) (i32.const 4)) (i32.const 2)))
      (func (export "f-cb") (param i32 i32 i32) (result i32)
        (i32.or (i32.shl (global.get $set) (i32.const 4)) (i32.const 2))))
    (core instance $i (instantiate $m (with "" (instance
      (export "waitable-set.new" (func $waitable-set.new))))))
    (func (export "f") async
      (canon lift (core func $i "f") async (callback (core func $i "f-cb")))))

  (component $B
    (import "f" (func $f async))
    (core func $f (canon lower (func $f) async))
    (core func $subtask.cancel (canon subtask.cancel))
    (core func $waitable-set.new (canon waitable-set.new))
    (core func $waitable.join (canon waitable.join))
    (core module $m
      (import "" "f" (func $f (result i32)))
      (import "" "subtask.cancel" (func $subtask.cancel (param i32) (result i32)))
      (import "" "waitable-set.new" (func $waitable-set.new (result i32)))
      (import "" "waitable.join" (func $waitable.join (param i32 i32)))
      (global $set (mut i32) (i32.const 0))
      (func (export "f") (result i32)
        ;; `$A.f` blocks: subtask 1, STARTED
        (if (i32.ne (call $f) (i32.const 0x11)) (then unreachable))
        ;; WAIT (cancellably) on subtask 1
        (global.set $set (call $waitable-set.new))
        (call $waitable.join (i32.const 1) (global.get $set))
        (i32.or (i32.shl (global.get $set) (i32.const 4)) (i32.const 2)))
      (func (export "f-cb") (param $ev i32) (param i32 i32) (result i32)
        ;; 6 == TASK_CANCELLED
        (if (i32.ne (local.get $ev) (i32.const 6)) (then unreachable))
        (call $waitable.join (i32.const 1) (i32.const 0))
        ;; `$A.f` never resolves, so this blocks forever
        (drop (call $subtask.cancel (i32.const 1)))
        unreachable))
    (core instance $i (instantiate $m (with "" (instance
      (export "f" (func $f))
      (export "subtask.cancel" (func $subtask.cancel))
      (export "waitable-set.new" (func $waitable-set.new))
      (export "waitable.join" (func $waitable.join))))))
    (func (export "f") async
      (canon lift (core func $i "f") async (callback (core func $i "f-cb")))))

  (component $C
    (import "f" (func $f async))
    (core func $f (canon lower (func $f) async))
    (core func $subtask.cancel (canon subtask.cancel))
    (core module $m
      (import "" "f" (func $f (result i32)))
      (import "" "subtask.cancel" (func $subtask.cancel (param i32) (result i32)))
      (func (export "g") (result i32)
        ;; `$B.f` blocks: subtask 1, STARTED
        (if (i32.ne (call $f) (i32.const 0x11)) (then unreachable))
        ;; `$B.f` never resolves, so this blocks forever
        (drop (call $subtask.cancel (i32.const 1)))
        unreachable)
      (func (export "g-cb") (param i32 i32 i32) (result i32) unreachable))
    (core instance $i (instantiate $m (with "" (instance
      (export "f" (func $f))
      (export "subtask.cancel" (func $subtask.cancel))))))
    (func (export "g") async
      (canon lift (core func $i "g") async (callback (core func $i "g-cb")))))

  (component $D
    (import "g" (func $g async))
    (core func $g (canon lower (func $g) async))
    (core func $task.return (canon task.return))
    (core module $m
      (import "" "g" (func $g (result i32)))
      (import "" "task.return" (func $task.return))
      (func (export "run") (result i32)
        ;; `$C.g` blocks in `subtask.cancel`: subtask 1, STARTED
        (if (i32.ne (call $g) (i32.const 0x11)) (then unreachable))
        (call $task.return)
        (i32.const 0 (; EXIT ;)))
      (func (export "run-cb") (param i32 i32 i32) (result i32) unreachable))
    (core instance $i (instantiate $m (with "" (instance
      (export "g" (func $g))
      (export "task.return" (func $task.return))))))
    (func (export "run") async
      (canon lift (core func $i "run") async (callback (core func $i "run-cb")))))

  (instance $a (instantiate $A))
  (instance $b (instantiate $B (with "f" (func $a "f"))))
  (instance $c (instantiate $C (with "f" (func $b "f"))))
  (instance $d (instantiate $D (with "g" (func $c "g"))))
  (export "run" (func $d "run")))

(assert_return (invoke "run"))
