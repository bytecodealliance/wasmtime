;;! component_model_async = true

;; $B.f is an `async` function lifted without `async`; it calls the sync-typed
;; $A.g twice through a sync-lowered import.  During the first call $A.g calls
;; a sync host import. The second call of $A.g blocks in `waitable-set.wait` with
;; no other thread of $A ready, which must trap.
(component
  (import "host-return-two" (func $host (result u32)))
  (component $A
    (import "host" (func $host (result u32)))
    (core func $host (canon lower (func $host)))
    (core module $libc (memory (export "mem") 1))
    (core instance $libc (instantiate $libc))
    (core func $waitable-set.new (canon waitable-set.new))
    (core func $waitable-set.wait (canon waitable-set.wait (memory (core memory $libc "mem"))))
    (core module $m
      (import "" "waitable-set.new" (func $waitable-set.new (result i32)))
      (import "" "waitable-set.wait" (func $waitable-set.wait (param i32 i32) (result i32)))
      (import "" "host" (func $host (result i32)))
      (global $calls (mut i32) (i32.const 0))
      (func (export "g")
        (global.set $calls (i32.add (global.get $calls) (i32.const 1)))
        (if (i32.eq (global.get $calls) (i32.const 1))
          (then (drop (call $host))))
        (if (i32.eq (global.get $calls) (i32.const 2))
          (then
            ;; nothing will ever be delivered: blocks
            (drop (call $waitable-set.wait (call $waitable-set.new) (i32.const 0)))
            unreachable))))
    (core instance $i (instantiate $m (with "" (instance
      (export "waitable-set.new" (func $waitable-set.new))
      (export "waitable-set.wait" (func $waitable-set.wait))
      (export "host" (func $host))))))
    (func (export "g") (canon lift (core func $i "g"))))

  (component $B
    (import "g" (func $g))
    (core func $g (canon lower (func $g)))
    (core module $m
      (import "" "g" (func $g))
      (func (export "f") (call $g) (call $g)))
    (core instance $i (instantiate $m (with "" (instance (export "g" (func $g))))))
    (func (export "f") async (canon lift (core func $i "f"))))

  (component $C
    (import "f" (func $f async))
    (core module $libc (memory (export "mem") 1))
    (core instance $libc (instantiate $libc))
    (core func $f (canon lower (func $f) async (memory (core memory $libc "mem"))))
    (core func $task.return (canon task.return))
    (core module $m
      (import "" "f" (func $f (result i32)))
      (import "" "task.return" (func $task.return))
      (func (export "run") (result i32)
        (drop (call $f))
        (call $task.return)
        (i32.const 0)) ;; EXIT
      (func (export "cb") (param i32 i32 i32) (result i32) unreachable))
    (core instance $i (instantiate $m (with "" (instance
      (export "f" (func $f))
      (export "task.return" (func $task.return))))))
    (func (export "run") async (canon lift (core func $i "run") async (callback (core func $i "cb")))))

  (instance $a (instantiate $A (with "host" (func $host))))
  (instance $b (instantiate $B (with "g" (func $a "g"))))
  (instance $c (instantiate $C (with "f" (func $b "f"))))
  (export "run" (func $c "run")))

(assert_trap (invoke "run") "cannot block a synchronous task before returning")
