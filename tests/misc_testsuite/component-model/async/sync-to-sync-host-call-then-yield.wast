;;! component_model_async = true

;; The sync-typed $R.run sync-calls $B.f (`async`, callback-lifted), which
;; sync-calls the sync-typed $A.g twice.  During the first call $A.g calls a
;; sync host import, then the second call of $A.g calls `thread.yield`, which
;; leaves it ready to run and is therefore a no-op for a sync-typed task.
(component
  (import "host-return-two" (func $host (result u32)))
  (component $A
    (import "host" (func $host (result u32)))
    (core func $host (canon lower (func $host)))
    (core func $yield (canon thread.yield))
    (core module $m
      (import "" "host" (func $host (result i32)))
      (import "" "yield" (func $yield (result i32)))
      (global $calls (mut i32) (i32.const 0))
      (func (export "g")
        (global.set $calls (i32.add (global.get $calls) (i32.const 1)))
        (if (i32.eq (global.get $calls) (i32.const 1)) (then (drop (call $host))))
        (if (i32.eq (global.get $calls) (i32.const 2)) (then (drop (call $yield))))))
    (core instance $i (instantiate $m (with "" (instance (export "host" (func $host)) (export "yield" (func $yield))))))
    (func (export "g") (canon lift (core func $i "g"))))
  (component $B
    (import "g" (func $g))
    (core func $g (canon lower (func $g)))
    (core func $task.return (canon task.return))
    (core module $m
      (import "" "g" (func $g))
      (import "" "task.return" (func $task.return))
      (func (export "f") (result i32) (call $g) (call $g) (call $task.return) (i32.const 0))
      (func (export "cb") (param i32 i32 i32) (result i32) unreachable))
    (core instance $i (instantiate $m (with "" (instance (export "g" (func $g)) (export "task.return" (func $task.return))))))
    (func (export "f") async (canon lift (core func $i "f") async (callback (core func $i "cb")))))
  (component $R
    (import "f" (func $f async))
    (core func $f (canon lower (func $f)))
    (core module $m
      (import "" "f" (func $f))
      (func (export "run") (call $f)))
    (core instance $i (instantiate $m (with "" (instance (export "f" (func $f))))))
    (func (export "run") (canon lift (core func $i "run"))))
  (instance $a (instantiate $A (with "host" (func $host))))
  (instance $b (instantiate $B (with "g" (func $a "g"))))
  (instance $r (instantiate $R (with "f" (func $b "f"))))
  (export "run" (func $r "run")))

(assert_return (invoke "run"))
