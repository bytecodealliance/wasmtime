;;! component_model_async = true
;;! component_model_threading = true

;; $A makes an async-lowered call to $J's callback-lifted "f", so $A's fiber is
;; parked waiting for "f" to suspend or return.  "f" then makes a sync-lowered
;; call to $K's sync-typed "g", which spawns a thread and yields to it.  When
;; that thread exits, $K has a sync-typed call in progress, so control must
;; switch back to "g"'s thread rather than to $A.  $A must only be resumed once
;; "f" returns.
(component
  (component $K
    (core module $libc (table (export "t") 1 funcref))
    (core instance $libc (instantiate $libc))
    (core type $ft (func (param i32)))
    (core func $thread.new-indirect (canon thread.new-indirect $ft (core table $libc "t")))
    (core func $thread.yield-then-resume (canon thread.yield-then-resume))
    (core module $m
      (import "" "thread.new-indirect" (func $new (param i32 i32) (result i32)))
      (import "" "thread.yield-then-resume" (func $ytr (param i32) (result i32)))
      (import "" "t" (table 1 funcref))
      (func $start (param i32))
      (elem (i32.const 0) func $start)
      (func (export "g") (result i32)
        (drop (call $ytr (call $new (i32.const 0) (i32.const 0))))
        (i32.const 42))
    )
    (core instance $i (instantiate $m (with "" (instance
      (export "thread.new-indirect" (func $thread.new-indirect))
      (export "thread.yield-then-resume" (func $thread.yield-then-resume))
      (export "t" (table $libc "t"))))))
    (func (export "g") (result u32) (canon lift (core func $i "g")))
  )
  (component $J
    (import "g" (func $g (result u32)))
    (core func $g (canon lower (func $g)))
    (core func $task.return (canon task.return (result u32)))
    (core module $m
      (import "" "g" (func $g (result i32)))
      (import "" "task.return" (func $ret (param i32)))
      (func (export "f") (result i32)
        (call $ret (call $g))
        (i32.const 0 (; EXIT ;)))
      (func (export "cb") (param i32 i32 i32) (result i32) unreachable)
    )
    (core instance $i (instantiate $m (with "" (instance
      (export "g" (func $g))
      (export "task.return" (func $task.return))))))
    (func (export "f") async (result u32)
      (canon lift (core func $i "f") async (callback (core func $i "cb"))))
  )
  (component $A
    (import "f" (func $f async (result u32)))
    (core module $libc (memory (export "memory") 1))
    (core instance $libc (instantiate $libc))
    (core func $f (canon lower (func $f) async (memory (core memory $libc "memory"))))
    (core func $task.return (canon task.return (result u32)))
    (core module $m
      (import "" "f" (func $f (param i32) (result i32)))
      (import "" "task.return" (func $ret (param i32)))
      (import "libc" "memory" (memory 1))
      (func (export "run") (result i32)
        ;; The subtask status should be RETURNED, and the result 42.
        (if (i32.ne (call $f (i32.const 0)) (i32.const 2 (; RETURNED ;)))
          (then unreachable))
        (call $ret (i32.load (i32.const 0)))
        (i32.const 0 (; EXIT ;)))
      (func (export "cb") (param i32 i32 i32) (result i32) unreachable)
    )
    (core instance $i (instantiate $m
      (with "" (instance
        (export "f" (func $f))
        (export "task.return" (func $task.return))))
      (with "libc" (instance $libc))))
    (func (export "run") async (result u32)
      (canon lift (core func $i "run") async (callback (core func $i "cb"))))
  )
  (instance $k (instantiate $K))
  (instance $j (instantiate $J (with "g" (func $k "g"))))
  (instance $a (instantiate $A (with "f" (func $j "f"))))
  (func (export "run") (alias export $a "run"))
)
(assert_return (invoke "run") (u32.const 42))
