;;! component_model_async = true
;;! component_model_more_async_builtins = true

;; A callback-lifted task returns `CALLBACK_CODE_YIELD`, queueing a work item to
;; call its callback again. Before that work item runs, the task's instance
;; becomes non-enterable because a sync-lifted, async-typed export of the same
;; instance is in progress, so the work item is parked until the instance can
;; be entered again. Cancelling the yielding task at that point must not be
;; delivered immediately but must instead be delivered once the instance is
;; enterable again.

;; Synchronous `subtask.cancel`: the cancellation is delivered once the
;; sync-lifted export finishes, after which the task acknowledges it.
(component
  (component $J
    (core func $task.cancel (canon task.cancel))
    (core func $thread.yield (canon thread.yield))
    (core module $m
      (import "" "task.cancel" (func $task.cancel))
      (import "" "thread.yield" (func $yield (result i32)))
      (func (export "x") (result i32)
        (i32.const 1 (; YIELD ;)))
      (func (export "x-cb") (param i32 i32 i32) (result i32)
        (if (i32.eq (local.get 0) (i32.const 6 (; TASK_CANCELLED ;)))
          (then
            (call $task.cancel)
            (return (i32.const 0 (; EXIT ;)))))
        (i32.const 1 (; YIELD ;)))
      (func (export "s")
        (local $i i32)
        (loop $l
          (drop (call $yield))
          (local.set $i (i32.add (local.get $i) (i32.const 1)))
          (br_if $l (i32.lt_u (local.get $i) (i32.const 10)))))
    )
    (core instance $i (instantiate $m (with "" (instance
      (export "task.cancel" (func $task.cancel))
      (export "thread.yield" (func $thread.yield))))))
    (func (export "x") async
      (canon lift (core func $i "x") async
        (callback (core func $i "x-cb"))))
    (func (export "s") async (canon lift (core func $i "s")))
  )
  (component $C
    (import "x" (func $x async))
    (import "s" (func $s async))
    (core func $x (canon lower (func $x) async))
    (core func $s (canon lower (func $s) async))
    (core func $subtask.cancel (canon subtask.cancel))
    (core func $thread.yield (canon thread.yield))
    (core module $m
      (import "" "x" (func $x (result i32)))
      (import "" "s" (func $s (result i32)))
      (import "" "subtask.cancel" (func $cancel (param i32) (result i32)))
      (import "" "thread.yield" (func $yield (result i32)))
      (func (export "run") (result i32)
        (local $hx i32) (local $hs i32)
        (local.set $hx (call $x))
        (if (i32.ne (i32.and (local.get $hx) (i32.const 0xf))
              (i32.const 1 (; STARTED ;)))
          (then unreachable))
        (local.set $hx (i32.shr_u (local.get $hx) (i32.const 4)))
        ;; make $J non-enterable
        (local.set $hs (call $s))
        (if (i32.ne (i32.and (local.get $hs) (i32.const 0xf))
              (i32.const 1 (; STARTED ;)))
          (then unreachable))
        ;; let the event loop park x's callback work item
        (drop (call $yield))
        (call $cancel (local.get $hx)))
    )
    (core instance $i (instantiate $m (with "" (instance
      (export "x" (func $x))
      (export "s" (func $s))
      (export "subtask.cancel" (func $subtask.cancel))
      (export "thread.yield" (func $thread.yield))))))
    (func (export "run") async (result u32) (canon lift (core func $i "run")))
  )
  (instance $j (instantiate $J))
  (instance $c (instantiate $C
    (with "x" (func $j "x"))
    (with "s" (func $j "s"))))
  (func (export "run") (alias export $c "run"))
)
(assert_return (invoke "run") (u32.const 4 (; RETURN_CANCELLED ;)))

;; Asynchronous `subtask.cancel`: the sync-lifted export never finishes, so the
;; cancellation can't be delivered and `subtask.cancel` returns `BLOCKED`.
(component
  (import "host" (instance $host (export "never-return" (func async))))
  (component $X
    (import "never-return" (func $never-return async))
    (core func $never-return (canon lower (func $never-return)))
    (core func $task-cancel (canon task.cancel))
    (core module $x
      (import "" "never-return" (func $never-return))
      (import "" "task.cancel" (func $task-cancel))
      (func (export "y") (result i32) (i32.const 1 (; YIELD ;)))
      (func (export "y-cb") (param i32 i32 i32) (result i32)
        (if (i32.eq (local.get 0) (i32.const 6 (; TASK_CANCELLED ;)))
          (then (call $task-cancel) (return (i32.const 0 (; EXIT ;)))))
        (i32.const 1 (; YIELD ;)))
      (func (export "blk") (call $never-return)))
    (core instance $x (instantiate $x (with "" (instance
      (export "never-return" (func $never-return))
      (export "task.cancel" (func $task-cancel))))))
    (func (export "y") async
      (canon lift (core func $x "y") async
        (callback (core func $x "y-cb"))))
    (func (export "blk") async (canon lift (core func $x "blk")))
  )
  (component $Y
    (import "x" (instance $x
      (export "y" (func async))
      (export "blk" (func async))))
    (core func $y (canon lower (func $x "y") async))
    (core func $blk (canon lower (func $x "blk") async))
    (core func $cancel (canon subtask.cancel async))
    (core func $task-return (canon task.return (result u32)))
    (core module $m
      (import "" "y" (func $y (result i32)))
      (import "" "blk" (func $blk (result i32)))
      (import "" "cancel" (func $cancel (param i32) (result i32)))
      (import "" "task.return" (func $task-return (param i32)))
      (global $hy (mut i32) (i32.const 0))
      (func (export "run") (result i32)
        (local $ret i32)
        (local.set $ret (call $y))
        (if (i32.ne (i32.and (local.get $ret) (i32.const 0xf))
              (i32.const 1 (; STARTED ;)))
          (then unreachable))
        (global.set $hy (i32.shr_u (local.get $ret) (i32.const 4)))
        ;; make $X non-enterable
        (if (i32.ne (i32.and (call $blk) (i32.const 0xf))
              (i32.const 1 (; STARTED ;)))
          (then unreachable))
        ;; let the event loop park y's callback work item
        (i32.const 1 (; YIELD ;)))
      (func (export "run-cb") (param i32 i32 i32) (result i32)
        (call $task-return (call $cancel (global.get $hy)))
        (i32.const 0 (; EXIT ;))))
    (core instance $m (instantiate $m (with "" (instance
      (export "y" (func $y))
      (export "blk" (func $blk))
      (export "cancel" (func $cancel))
      (export "task.return" (func $task-return))))))
    (func (export "run") async (result u32)
      (canon lift (core func $m "run") async
        (callback (core func $m "run-cb"))))
  )
  (instance $x (instantiate $X
    (with "never-return" (func $host "never-return"))))
  (instance $y (instantiate $Y (with "x" (instance $x))))
  (export "run" (func $y "run"))
)
(assert_return (invoke "run") (u32.const 0xffffffff (; BLOCKED ;)))
