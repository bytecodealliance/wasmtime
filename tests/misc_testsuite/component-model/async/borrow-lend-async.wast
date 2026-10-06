;;! component_model_async = true

;; A lends an own handle to an async callee B which holds the borrow across
;; yields, waits, and cancellation.
;;
;; mode 0: B drops the borrow and returns immediately
;; mode 1: B returns without dropping the borrow (traps)
;; mode 2: B yields, then drops the borrow and returns
;; mode 3: B yields, and A drops the own handle while it's lent (traps)
;; mode 4: B waits, is cancelled, and cancels without dropping the borrow
;;         (traps)
;; mode 5: B waits, is cancelled, and drops the borrow before cancelling

(component definition $C
  (import "host" (instance $host
    (export "resource1" (type $r (sub resource)))
    (export "[constructor]resource1" (func (param "r" u32) (result (own $r))))
  ))
  (alias export $host "resource1" (type $r))
  (alias export $host "[constructor]resource1" (func $ctor))

  (component $B
    (import "r" (type $r (sub resource)))
    (core func $resource.drop (canon resource.drop $r))
    (core func $task.return (canon task.return))
    (core func $task.cancel (canon task.cancel))
    (core func $waitable-set.new (canon waitable-set.new))
    (core module $m
      (import "" "resource.drop" (func $resource.drop (param i32)))
      (import "" "task.return" (func $task.return))
      (import "" "task.cancel" (func $task.cancel))
      (import "" "waitable-set.new" (func $waitable-set.new (result i32)))
      (global $b (mut i32) (i32.const 0))
      (global $mode (mut i32) (i32.const 0))
      (func (export "f") (param $x i32) (param $mode i32) (result i32)
        (global.set $b (local.get $x))
        ;; mode 0: drop + return
        (if (i32.eq (local.get $mode) (i32.const 0)) (then
          (call $resource.drop (local.get $x))
          (call $task.return)
          (return (i32.const 0 (; EXIT ;)))))
        ;; mode 1: return without dropping borrow -> must trap
        (if (i32.eq (local.get $mode) (i32.const 1)) (then
          (call $task.return)
          (return (i32.const 0 (; EXIT ;)))))
        (global.set $mode (local.get $mode))
        ;; mode 4/5: wait forever (until cancelled) holding borrow
        (if (i32.ge_u (local.get $mode) (i32.const 4)) (then
          (return
            (i32.or
              (i32.const 2 (; WAIT ;))
              (i32.shl (call $waitable-set.new) (i32.const 4))))))
        ;; mode 2/3: yield while holding borrow
        (i32.const 1 (; YIELD ;)))
      (func (export "cb") (param $ev i32) (param i32 i32) (result i32)
        (if (i32.ge_u (global.get $mode) (i32.const 4)) (then
          (if (i32.ne (local.get $ev) (i32.const 6 (; TASK_CANCELLED ;)))
            (then unreachable))
          (if (i32.eq (global.get $mode) (i32.const 5)) (then
            (call $resource.drop (global.get $b))))
          (call $task.cancel)
          (return (i32.const 0 (; EXIT ;)))))
        (call $resource.drop (global.get $b))
        (call $task.return)
        (i32.const 0 (; EXIT ;)))
    )
    (core instance $i (instantiate $m (with "" (instance
      (export "resource.drop" (func $resource.drop))
      (export "task.return" (func $task.return))
      (export "task.cancel" (func $task.cancel))
      (export "waitable-set.new" (func $waitable-set.new))
    ))))
    (func (export "f") async (param "x" (borrow $r)) (param "mode" u32)
      (canon lift (core func $i "f") async (callback (core func $i "cb"))))
  )
  (instance $b (instantiate $B (with "r" (type $r))))

  (component $A
    (import "r" (type $r (sub resource)))
    (import "ctor" (func $ctor (param "r" u32) (result (own $r))))
    (import "f" (func $f async (param "x" (borrow $r)) (param "mode" u32)))
    (core func $ctor (canon lower (func $ctor)))
    (core func $f (canon lower (func $f) async))
    (core func $resource.drop (canon resource.drop $r))
    (core func $waitable-set.new (canon waitable-set.new))
    (core func $waitable.join (canon waitable.join))
    (core func $subtask.drop (canon subtask.drop))
    (core func $task.return (canon task.return))
    (core func $subtask.cancel (canon subtask.cancel))
    (core module $m
      (import "" "ctor" (func $ctor (param i32) (result i32)))
      (import "" "f" (func $f (param i32 i32) (result i32)))
      (import "" "resource.drop" (func $resource.drop (param i32)))
      (import "" "waitable-set.new" (func $waitable-set.new (result i32)))
      (import "" "waitable.join" (func $waitable.join (param i32 i32)))
      (import "" "subtask.drop" (func $subtask.drop (param i32)))
      (import "" "task.return" (func $task.return))
      (import "" "subtask.cancel"
        (func $subtask.cancel (param i32) (result i32)))
      (global $h (mut i32) (i32.const 0))
      (global $s (mut i32) (i32.const 0))
      (func (export "run") (param $mode i32) (result i32)
        (local $h i32) (local $s i32) (local $ws i32)
        (local.set $h (call $ctor (i32.const 7)))
        (local.set $s (call $f (local.get $h) (local.get $mode)))
        (if (i32.eq (i32.and (local.get $s) (i32.const 0xf))
              (i32.const 1 (; STARTED ;))) (then
          (local.set $s (i32.shr_u (local.get $s) (i32.const 4)))
          ;; mode 3: drop the own while it's lent -> must trap
          (if (i32.eq (local.get $mode) (i32.const 3)) (then
            (call $resource.drop (local.get $h))))
          (if (i32.ge_u (local.get $mode) (i32.const 4)) (then
            (if (i32.ne (call $subtask.cancel (local.get $s))
                  (i32.const 4 (; RETURN_CANCELLED ;)))
              (then unreachable))
            (call $subtask.drop (local.get $s))
            (call $resource.drop (local.get $h))
            (call $task.return)
            (return (i32.const 0 (; EXIT ;)))))
          ;; mode 2: wait for the subtask to return, finishing in `cb`
          (global.set $h (local.get $h))
          (global.set $s (local.get $s))
          (local.set $ws (call $waitable-set.new))
          (call $waitable.join (local.get $s) (local.get $ws))
          (return
            (i32.or
              (i32.const 2 (; WAIT ;))
              (i32.shl (local.get $ws) (i32.const 4))))))
        (call $resource.drop (local.get $h))
        (call $task.return)
        (i32.const 0 (; EXIT ;)))
      (func (export "cb")
        (param $ev i32) (param $idx i32) (param $code i32) (result i32)
        (if (i32.ne (local.get $ev) (i32.const 1 (; SUBTASK ;)))
          (then unreachable))
        (if (i32.ne (local.get $idx) (global.get $s)) (then unreachable))
        (if (i32.ne (local.get $code) (i32.const 2 (; RETURNED ;)))
          (then unreachable))
        (call $subtask.drop (global.get $s))
        (call $resource.drop (global.get $h))
        (call $task.return)
        (i32.const 0 (; EXIT ;)))
    )
    (core instance $i (instantiate $m (with "" (instance
      (export "ctor" (func $ctor))
      (export "f" (func $f))
      (export "resource.drop" (func $resource.drop))
      (export "waitable-set.new" (func $waitable-set.new))
      (export "waitable.join" (func $waitable.join))
      (export "subtask.drop" (func $subtask.drop))
      (export "task.return" (func $task.return))
      (export "subtask.cancel" (func $subtask.cancel))
    ))))
    (func (export "run") async (param "mode" u32)
      (canon lift (core func $i "run") async (callback (core func $i "cb"))))
  )
  (instance $a (instantiate $A
    (with "r" (type $r))
    (with "ctor" (func $ctor))
    (with "f" (func $b "f"))))
  (func (export "run") (alias export $a "run"))
)

(component instance $c0 $C)
(assert_return (invoke "run" (u32.const 0)))
(component instance $c1 $C)
(assert_trap (invoke "run" (u32.const 1)) "borrow handles still remain")
(component instance $c2 $C)
(assert_return (invoke "run" (u32.const 2)))
(component instance $c3 $C)
(assert_trap (invoke "run" (u32.const 3))
  "cannot remove owned resource while borrowed")
(component instance $c4 $C)
(assert_trap (invoke "run" (u32.const 4)) "borrow handles still remain")
(component instance $c5 $C)
(assert_return (invoke "run" (u32.const 5)))
