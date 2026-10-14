;;! component_model_async = true

;; A lends own<r> `h` to B (async). B re-lends its borrow `b` to C (async),
;; then drops `b` while C's subtask is still unresolved and still holds a
;; borrow derived from it. Per the canonical ABI `lift_borrow` adds a lender
;; for *any* handle, and `resource.drop` traps if the handle has any
;; outstanding lends, so B's `resource.drop` must trap. If it didn't then B
;; could return, A would destroy the resource, and C would still be able to use
;; its borrow of a destroyed resource.
(component
  (import "host" (instance $host
    (export "resource1" (type $r (sub resource)))
    (export "[constructor]resource1" (func (param "r" u32) (result (own $r))))
  ))
  (alias export $host "resource1" (type $r))
  (alias export $host "[constructor]resource1" (func $ctor))

  ;; C: holds a borrow across a yield.
  (component $C
    (import "r" (type $r (sub resource)))
    (core module $m
      (func (export "g") (param i32) (result i32)
        (i32.const 1 (; YIELD ;)))
      (func (export "cb") (param i32 i32 i32) (result i32) unreachable))
    (core instance $i (instantiate $m))
    (func (export "g") async (param "x" (borrow $r))
      (canon lift (core func $i "g") async (callback (core func $i "cb"))))
  )
  (instance $c (instantiate $C (with "r" (type $r))))

  ;; B: re-lends its borrow to C, then drops it, which traps.
  (component $B
    (import "r" (type $r (sub resource)))
    (import "g" (func $g async (param "x" (borrow $r))))
    (core func $g (canon lower (func $g) async))
    (core func $resource.drop (canon resource.drop $r))
    (core module $m
      (import "" "g" (func $g (param i32) (result i32)))
      (import "" "resource.drop" (func $resource.drop (param i32)))
      (func (export "f") (param $x i32) (result i32)
        ;; C starts and yields, so this must be STARTED
        (if (i32.ne (i32.and (call $g (local.get $x)) (i32.const 0xf))
              (i32.const 1 (; STARTED ;)))
          (then unreachable))
        ;; `x` is lent to C's unresolved subtask, so this traps
        (call $resource.drop (local.get $x))
        unreachable)
      (func (export "cb") (param i32 i32 i32) (result i32) unreachable))
    (core instance $i (instantiate $m (with "" (instance
      (export "g" (func $g))
      (export "resource.drop" (func $resource.drop))))))
    (func (export "f") async (param "x" (borrow $r))
      (canon lift (core func $i "f") async (callback (core func $i "cb"))))
  )
  (instance $b (instantiate $B (with "r" (type $r)) (with "g" (func $c "g"))))

  ;; A: lends an own handle to B.
  (component $A
    (import "r" (type $r (sub resource)))
    (import "ctor" (func $ctor (param "r" u32) (result (own $r))))
    (import "f" (func $f async (param "x" (borrow $r))))
    (core func $ctor (canon lower (func $ctor)))
    (core func $f (canon lower (func $f) async))
    (core module $m
      (import "" "ctor" (func $ctor (param i32) (result i32)))
      (import "" "f" (func $f (param i32) (result i32)))
      (func (export "run") (result i32)
        (drop (call $f (call $ctor (i32.const 42))))
        unreachable)
      (func (export "cb") (param i32 i32 i32) (result i32) unreachable))
    (core instance $i (instantiate $m (with "" (instance
      (export "ctor" (func $ctor))
      (export "f" (func $f))))))
    (func (export "run") async
      (canon lift (core func $i "run") async (callback (core func $i "cb"))))
  )
  (instance $a (instantiate $A
    (with "r" (type $r))
    (with "ctor" (func $ctor))
    (with "f" (func $b "f"))))
  (func (export "run") (alias export $a "run"))
)

(assert_trap (invoke "run")
  "cannot remove borrowed resource while it is lent out")

;; Variations on the above where B re-lends its borrow to C and then drops its
;; borrow only once C's lend has been released, which is all valid.
(component definition $C
  (import "host" (instance $host
    (export "resource1" (type $r (sub resource)))
    (export "[constructor]resource1" (func (param "r" u32) (result (own $r))))
    (export "[static]resource1.drops" (func (result u32)))
  ))
  (alias export $host "resource1" (type $r))
  (alias export $host "[constructor]resource1" (func $ctor))
  (alias export $host "[static]resource1.drops" (func $drops))

  ;; C: takes a borrow and holds it until it's resumed or cancelled.
  ;;
  ;; mode 0: YIELD, then drop the borrow and return
  ;; mode 1: WAIT forever, and when cancelled drop the borrow and cancel
  ;; mode 2: WAIT forever, and when cancelled cancel without dropping the
  ;;         borrow (traps)
  (component $C
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
      (func (export "g") (param $x i32) (param $mode i32) (result i32)
        (global.set $b (local.get $x))
        (global.set $mode (local.get $mode))
        (if (i32.eqz (local.get $mode)) (then (return (i32.const 1)))) ;; YIELD
        ;; WAIT on an empty waitable set
        (i32.or (i32.const 2) (i32.shl (call $waitable-set.new) (i32.const 4))))
      (func (export "cb") (param $ev i32) (param i32 i32) (result i32)
        (if (i32.eqz (global.get $mode)) (then
          (call $resource.drop (global.get $b))
          (call $task.return)
          (return (i32.const 0))))
        ;; EVENT_TASK_CANCELLED
        (if (i32.ne (local.get $ev) (i32.const 6)) (then unreachable))
        (if (i32.eq (global.get $mode) (i32.const 1)) (then
          (call $resource.drop (global.get $b))))
        (call $task.cancel)
        (i32.const 0))
    )
    (core instance $i (instantiate $m (with "" (instance
      (export "resource.drop" (func $resource.drop))
      (export "task.return" (func $task.return))
      (export "task.cancel" (func $task.cancel))
      (export "waitable-set.new" (func $waitable-set.new))
    ))))
    (func (export "g") async (param "x" (borrow $r)) (param "mode" u32)
      (canon lift (core func $i "g") async (callback (core func $i "cb"))))
  )
  (instance $c (instantiate $C (with "r" (type $r))))

  ;; B: re-lends its borrow to C and drops it after C has resolved.
  ;;
  ;; mode 0: async call to C which yields, wait for it to return
  ;; mode 1: async call to C, cancel it, and C drops its borrow
  ;; mode 2: async call to C, cancel it, and C keeps its borrow (traps)
  ;; mode 3: sync call to C
  (component $B
    (import "r" (type $r (sub resource)))
    (import "g" (func $g async (param "x" (borrow $r)) (param "mode" u32)))
    (core func $g.async (canon lower (func $g) async))
    (core func $g.sync (canon lower (func $g)))
    (core func $resource.drop (canon resource.drop $r))
    (core func $task.return (canon task.return))
    (core func $waitable-set.new (canon waitable-set.new))
    (core func $waitable.join (canon waitable.join))
    (core func $subtask.drop (canon subtask.drop))
    (core func $subtask.cancel (canon subtask.cancel))
    (core module $m
      (import "" "g.async" (func $g.async (param i32 i32) (result i32)))
      (import "" "g.sync" (func $g.sync (param i32 i32)))
      (import "" "resource.drop" (func $resource.drop (param i32)))
      (import "" "task.return" (func $task.return))
      (import "" "waitable-set.new" (func $waitable-set.new (result i32)))
      (import "" "waitable.join" (func $waitable.join (param i32 i32)))
      (import "" "subtask.drop" (func $subtask.drop (param i32)))
      (import "" "subtask.cancel"
        (func $subtask.cancel (param i32) (result i32)))
      (global $x (mut i32) (i32.const 0))
      (global $s (mut i32) (i32.const 0))
      (func (export "f") (param $x i32) (param $mode i32) (result i32)
        (local $s i32) (local $ws i32)
        (global.set $x (local.get $x))

        (if (i32.eq (local.get $mode) (i32.const 3)) (then
          (call $g.sync (local.get $x) (i32.const 0))
          (call $resource.drop (local.get $x))
          (call $task.return)
          (return (i32.const 0))))

        (local.set $s (call $g.async (local.get $x) (local.get $mode)))
        ;; STARTED
        (if (i32.ne (i32.and (local.get $s) (i32.const 0xf)) (i32.const 1))
          (then unreachable))
        (local.set $s (i32.shr_u (local.get $s) (i32.const 4)))
        (global.set $s (local.get $s))

        (if (i32.eqz (local.get $mode)) (then
          (local.set $ws (call $waitable-set.new))
          (call $waitable.join (local.get $s) (local.get $ws))
          ;; WAIT for C to return
          (return ;; WAIT
            (i32.or (i32.const 2) (i32.shl (local.get $ws) (i32.const 4))))))

        ;; RETURN_CANCELLED
        (if (i32.ne (call $subtask.cancel (local.get $s)) (i32.const 4))
          (then unreachable))
        (call $subtask.drop (local.get $s))
        (call $resource.drop (local.get $x))
        (call $task.return)
        (i32.const 0))
      (func (export "cb")
        (param $ev i32) (param $s i32) (param $status i32) (result i32)
        ;; EVENT_SUBTASK for C with RETURNED
        (if (i32.ne (local.get $ev) (i32.const 1)) (then unreachable))
        (if (i32.ne (local.get $s) (global.get $s)) (then unreachable))
        (if (i32.ne (local.get $status) (i32.const 2)) (then unreachable))
        (call $subtask.drop (global.get $s))
        (call $resource.drop (global.get $x))
        (call $task.return)
        (i32.const 0))
    )
    (core instance $i (instantiate $m (with "" (instance
      (export "g.async" (func $g.async))
      (export "g.sync" (func $g.sync))
      (export "resource.drop" (func $resource.drop))
      (export "task.return" (func $task.return))
      (export "waitable-set.new" (func $waitable-set.new))
      (export "waitable.join" (func $waitable.join))
      (export "subtask.drop" (func $subtask.drop))
      (export "subtask.cancel" (func $subtask.cancel))
    ))))
    (func (export "f") async (param "x" (borrow $r)) (param "mode" u32)
      (canon lift (core func $i "f") async (callback (core func $i "cb"))))
  )
  (instance $b (instantiate $B (with "r" (type $r)) (with "g" (func $c "g"))))

  ;; A: lends an own handle to B, then destroys it and returns how many
  ;; resources were destroyed during the call.
  (component $A
    (import "r" (type $r (sub resource)))
    (import "ctor" (func $ctor (param "r" u32) (result (own $r))))
    (import "drops" (func $drops (result u32)))
    (import "f" (func $f async (param "x" (borrow $r)) (param "mode" u32)))
    (core func $ctor (canon lower (func $ctor)))
    (core func $drops (canon lower (func $drops)))
    (core func $f (canon lower (func $f)))
    (core func $resource.drop (canon resource.drop $r))
    (core func $task.return (canon task.return (result u32)))
    (core module $m
      (import "" "ctor" (func $ctor (param i32) (result i32)))
      (import "" "drops" (func $drops (result i32)))
      (import "" "f" (func $f (param i32 i32)))
      (import "" "resource.drop" (func $resource.drop (param i32)))
      (import "" "task.return" (func $task.return (param i32)))
      (func (export "run") (param $mode i32) (result i32)
        (local $h i32) (local $before i32)
        (local.set $before (call $drops))
        (local.set $h (call $ctor (i32.const 42)))
        (call $f (local.get $h) (local.get $mode))
        (call $resource.drop (local.get $h))
        (call $task.return (i32.sub (call $drops) (local.get $before)))
        (i32.const 0))
      (func (export "cb") (param i32 i32 i32) (result i32) unreachable)
    )
    (core instance $i (instantiate $m (with "" (instance
      (export "ctor" (func $ctor))
      (export "drops" (func $drops))
      (export "f" (func $f))
      (export "resource.drop" (func $resource.drop))
      (export "task.return" (func $task.return))
    ))))
    (func (export "run") async (param "mode" u32) (result u32)
      (canon lift (core func $i "run") async (callback (core func $i "cb"))))
  )
  (instance $a (instantiate $A (with "r" (type $r)) (with "ctor" (func $ctor))
    (with "drops" (func $drops)) (with "f" (func $b "f"))))
  (func (export "run") (alias export $a "run"))
)

(component instance $c0 $C)
(assert_return (invoke "run" (u32.const 0)) (u32.const 1))
(component instance $c1 $C)
(assert_return (invoke "run" (u32.const 1)) (u32.const 1))
(component instance $c2 $C)
(assert_trap (invoke "run" (u32.const 2)) "borrow handles still remain")
(component instance $c3 $C)
(assert_return (invoke "run" (u32.const 3)) (u32.const 1))

;; Same as the first test above, but B re-lends its borrow to a host subtask
;; instead of a guest subtask.
(component definition $C
  (import "host" (instance $host
    (export "resource1" (type $r (sub resource)))
    (export "[constructor]resource1" (func (param "r" u32) (result (own $r))))
    (export "[method]resource1.never-return"
      (func async (param "self" (borrow $r))))
  ))
  (alias export $host "resource1" (type $r))
  (alias export $host "[constructor]resource1" (func $ctor))
  (alias export $host "[method]resource1.never-return" (func $never-return))

  ;; B: re-lends its borrow to the host.
  ;;
  ;; mode 0: drop the borrow while the host subtask is still running (traps)
  ;; mode 1: cancel the host subtask and then drop the borrow
  (component $B
    (import "r" (type $r (sub resource)))
    (import "never-return"
      (func $never-return async (param "self" (borrow $r))))
    (core func $never-return (canon lower (func $never-return) async))
    (core func $resource.drop (canon resource.drop $r))
    (core func $task.return (canon task.return))
    (core func $subtask.drop (canon subtask.drop))
    (core func $subtask.cancel (canon subtask.cancel))
    (core module $m
      (import "" "never-return" (func $never-return (param i32) (result i32)))
      (import "" "resource.drop" (func $resource.drop (param i32)))
      (import "" "task.return" (func $task.return))
      (import "" "subtask.drop" (func $subtask.drop (param i32)))
      (import "" "subtask.cancel"
        (func $subtask.cancel (param i32) (result i32)))
      (func (export "f") (param $x i32) (param $mode i32) (result i32)
        (local $s i32)
        (local.set $s (call $never-return (local.get $x)))
        ;; STARTED
        (if (i32.ne (i32.and (local.get $s) (i32.const 0xf)) (i32.const 1))
          (then unreachable))
        (local.set $s (i32.shr_u (local.get $s) (i32.const 4)))
        (if (local.get $mode) (then
          ;; RETURN_CANCELLED
          (if (i32.ne (call $subtask.cancel (local.get $s)) (i32.const 4))
            (then unreachable))
          (call $subtask.drop (local.get $s))))
        (call $resource.drop (local.get $x))
        (call $task.return)
        (i32.const 0))
      (func (export "cb") (param i32 i32 i32) (result i32) unreachable)
    )
    (core instance $i (instantiate $m (with "" (instance
      (export "never-return" (func $never-return))
      (export "resource.drop" (func $resource.drop))
      (export "task.return" (func $task.return))
      (export "subtask.drop" (func $subtask.drop))
      (export "subtask.cancel" (func $subtask.cancel))
    ))))
    (func (export "f") async (param "x" (borrow $r)) (param "mode" u32)
      (canon lift (core func $i "f") async (callback (core func $i "cb"))))
  )
  (instance $b (instantiate $B (with "r" (type $r))
    (with "never-return" (func $never-return))))

  ;; A: lends an own handle to B and then destroys it.
  (component $A
    (import "r" (type $r (sub resource)))
    (import "ctor" (func $ctor (param "r" u32) (result (own $r))))
    (import "f" (func $f async (param "x" (borrow $r)) (param "mode" u32)))
    (core func $ctor (canon lower (func $ctor)))
    (core func $f (canon lower (func $f)))
    (core func $resource.drop (canon resource.drop $r))
    (core func $task.return (canon task.return))
    (core module $m
      (import "" "ctor" (func $ctor (param i32) (result i32)))
      (import "" "f" (func $f (param i32 i32)))
      (import "" "resource.drop" (func $resource.drop (param i32)))
      (import "" "task.return" (func $task.return))
      (func (export "run") (param $mode i32) (result i32)
        (local $h i32)
        (local.set $h (call $ctor (i32.const 42)))
        (call $f (local.get $h) (local.get $mode))
        (call $resource.drop (local.get $h))
        (call $task.return)
        (i32.const 0))
      (func (export "cb") (param i32 i32 i32) (result i32) unreachable)
    )
    (core instance $i (instantiate $m (with "" (instance
      (export "ctor" (func $ctor))
      (export "f" (func $f))
      (export "resource.drop" (func $resource.drop))
      (export "task.return" (func $task.return))
    ))))
    (func (export "run") async (param "mode" u32)
      (canon lift (core func $i "run") async (callback (core func $i "cb"))))
  )
  (instance $a (instantiate $A (with "r" (type $r)) (with "ctor" (func $ctor))
    (with "f" (func $b "f"))))
  (func (export "run") (alias export $a "run"))
)

(component instance $c0 $C)
(assert_trap (invoke "run" (u32.const 0))
  "cannot remove borrowed resource while it is lent out")
(component instance $c1 $C)
(assert_return (invoke "run" (u32.const 1)))
