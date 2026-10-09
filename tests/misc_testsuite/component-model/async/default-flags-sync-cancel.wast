;;! component_model_async = true
;;! component_model_more_async_builtins = true

;; $C async-lowers a call to $B.h.  $B, before it has suspended, makes a
;; sync-lowered call to the sync-lifted $A.g (which runs to completion on
;; $B's fiber) and then yields to one of its own subtasks by cancelling the
;; started, yielding $A.f.
;;
;; definitions.py: nothing special happens; $B's cancel resolves $A.f as
;; CANCELLED_BEFORE_RETURNED, $B returns, $C sees RETURNED (2).
;;
;; Wasmtime: when $A.g's thread exits, cleanup_thread calls
;; take_next_switch_item(), which moves the event loop's next_switch_item
;; (the fiber of $C, parked in start_call until $B first suspends) into
;; switch_item.  When $B then suspends with YieldingToSubtask (the cancel's
;; yield_), resume_fiber stores $B's fiber as the new next_switch_item, the
;; stale switch item resumes $C inside start_call, and the save/restore in
;; StoreOpaque::suspend overwrites next_switch_item with the value it saved,
;; dropping $B's in-progress fiber: the `StoreFiber` Drop impl asserts, the
;; raw fiber's Drop panics again during unwinding, and the process aborts.
(component
  (component $A
    (core func $task.cancel (canon task.cancel))
    (core func $ws.new (canon waitable-set.new))
    (core func $ws.drop (canon waitable-set.drop))
    (core module $m
      (import "" "task.cancel" (func $task.cancel))
      (import "" "waitable-set.new" (func $ws.new (result i32)))
      (import "" "waitable-set.drop" (func $ws.drop (param i32)))
      ;; f: yield on start (callback code 1)
      (func (export "f") (result i32) (i32.const 1))
      ;; cb: on TASK_CANCELLED (6) cancel the task and exit; anything else is unexpected
      (func (export "cb") (param i32 i32 i32) (result i32)
        (if (i32.ne (local.get 0) (i32.const 6)) (then unreachable))
        (call $task.cancel)
        (i32.const 0))
      ;; g: a sync export that touches the concurrent state (any built-in or
      ;; host call does; the fuzzer used resource.new), which makes Wasmtime
      ;; materialize the deferred `enter_guest_sync_call` for this call and
      ;; therefore run `cleanup_thread` when it exits
      (func (export "g") (call $ws.drop (call $ws.new))))
    (core instance $i (instantiate $m (with "" (instance
      (export "task.cancel" (func $task.cancel))
      (export "waitable-set.new" (func $ws.new))
      (export "waitable-set.drop" (func $ws.drop))))))
    (func (export "f") async (canon lift (core func $i "f") async (callback (core func $i "cb"))))
    (func (export "g") (canon lift (core func $i "g"))))

  (component $B
    (import "f" (func $f async))
    (import "g" (func $g))
    (core func $f (canon lower (func $f) async))
    (core func $g (canon lower (func $g)))
    (core func $subtask.cancel (canon subtask.cancel))
    (core func $subtask.drop (canon subtask.drop))
    (core func $task.return (canon task.return))
    (core module $m
      (import "" "f" (func $f (result i32)))
      (import "" "g" (func $g))
      (import "" "subtask.cancel" (func $subtask.cancel (param i32) (result i32)))
      (import "" "subtask.drop" (func $subtask.drop (param i32)))
      (import "" "task.return" (func $task.return))
      (func (export "h") (result i32) (local $r i32)
        (local.set $r (call $f))
        ;; the call must be STARTED (1) with a subtask handle
        (if (i32.ne (i32.and (local.get $r) (i32.const 0xf)) (i32.const 1)) (then unreachable))
        ;; a sync call to a sync-lifted export, made before this task ever suspended
        (call $g)
        ;; cancel the yielding subtask: definitions.py returns CANCELLED_BEFORE_RETURNED (4)
        (if (i32.ne (call $subtask.cancel (i32.shr_u (local.get $r) (i32.const 4))) (i32.const 4)) (then unreachable))
        (call $subtask.drop (i32.shr_u (local.get $r) (i32.const 4)))
        (call $task.return)
        (i32.const 0 (; EXIT ;)))
      (func (export "cb") (param i32 i32 i32) (result i32) unreachable))
    (core instance $i (instantiate $m (with "" (instance
      (export "f" (func $f))
      (export "g" (func $g))
      (export "subtask.cancel" (func $subtask.cancel))
      (export "subtask.drop" (func $subtask.drop))
      (export "task.return" (func $task.return))))))
    (func (export "h") async (canon lift (core func $i "h") async (callback (core func $i "cb")))))

  (component $C
    (import "h" (func $h async))
    (core func $h (canon lower (func $h) async))
    (core func $task.return (canon task.return (result u32)))
    (core module $m
      (import "" "h" (func $h (result i32)))
      (import "" "task.return" (func $task.return (param i32)))
      (func (export "main") (result i32)
        ;; $B runs to completion before $C regains control: RETURNED (2), no handle
        (call $task.return (call $h))
        (i32.const 0 (; EXIT ;)))
      (func (export "cb") (param i32 i32 i32) (result i32) unreachable))
    (core instance $i (instantiate $m (with "" (instance
      (export "h" (func $h))
      (export "task.return" (func $task.return))))))
    (func (export "main") async (result u32) (canon lift (core func $i "main") async (callback (core func $i "cb")))))

  (instance $a (instantiate $A))
  (instance $b (instantiate $B (with "f" (func $a "f")) (with "g" (func $a "g"))))
  (instance $c (instantiate $C (with "h" (func $b "h"))))
  (export "main" (func $c "main")))

(assert_return (invoke "main") (u32.const 2))
