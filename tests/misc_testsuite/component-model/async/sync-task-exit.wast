;;! component_model_async = true
;;! bulk_memory = true

;; The following are regression tests to ensure that sync-to-sync calls from
;; async calls aren't confused about the next work item to run, as was the case
;; in earlier versions of the runtime.

(component
  (core module $shim
    (table (export "funcs") 1 1 funcref)
    (func (export "export") (result i32)
      (call_indirect (result i32) (i32.const 0))))
  (core instance $shim (instantiate $shim))
  (func $shim-export (result u32) (canon lift (core func $shim "export")))

  (component $X
    (import "import" (func $import (result u32)))
    (core func $import (canon lower (func $import)))
    (core func $waitable-set.new (canon waitable-set.new))
    (core func $waitable-set.drop (canon waitable-set.drop))
    (core func $task.return (canon task.return))
    (core module $m
      (import "" "import" (func $import (result i32)))
      (import "" "waitable-set.new" (func $waitable-set.new (result i32)))
      (import "" "waitable-set.drop" (func $waitable-set.drop (param i32)))
      (import "" "task.return" (func $task.return))
      (func (export "s") (result i32) (call $import))
      (func (export "h") (result i32)
        (call $waitable-set.drop (call $waitable-set.new))
        (i32.const 7))
      (func (export "y") (result i32) (i32.const 1 (; YIELD ;)))
      (func (export "y-cb") (param i32 i32 i32) (result i32)
        (call $task.return)
        (i32.const 0 (; EXIT ;)))
      (func (export "z") (result i32)
        (call $task.return) (i32.const 0 (; EXIT ;))))
    (core instance $i (instantiate $m
      (with "" (instance
        (export "import" (func $import))
        (export "waitable-set.new" (func $waitable-set.new))
        (export "waitable-set.drop" (func $waitable-set.drop))
        (export "task.return" (func $task.return))))))
    (func (export "s") (result u32) (canon lift (core func $i "s")))
    (func (export "h") async (result u32) (canon lift (core func $i "h")))
    (func (export "y") async
      (canon lift (core func $i "y") async
        (callback (core func $i "y-cb"))))
    (func (export "z") async
      (canon lift (core func $i "z") async
        (callback (core func $i "y-cb")))))
  (instance $x (instantiate $X (with "import" (func $shim-export))))

  (core func $x-h (canon lower (func $x "h")))
  (core module $donut
    (import "" "funcs" (table 1 1 funcref))
    (import "" "h" (func $h (result i32)))
    (func $guest-export (result i32) (call $h))
    (elem declare func $guest-export)
    (func $start (table.set (i32.const 0) (ref.func $guest-export)))
    (start $start))
  (core instance $donut (instantiate $donut
    (with "" (instance
      (export "h" (func $x-h))
      (export "funcs" (table $shim "funcs"))))))

  (component $D
    (import "x" (instance $x
      (export "s" (func (result u32)))
      (export "y" (func async))
      (export "z" (func async))))
    (core func $s (canon lower (func $x "s")))
    (core func $y (canon lower (func $x "y") async))
    (core func $z (canon lower (func $x "z") async))
    (core func $task.return (canon task.return (result u32)))
    (core module $d
      (import "" "s" (func $s (result i32)))
      (import "" "y" (func $y (result i32)))
      (import "" "z" (func $z (result i32)))
      (import "" "task.return" (func $task.return (param i32)))
      (func (export "run") (result i32)
        (drop (call $y))
        (drop (call $s))
        (call $task.return (call $z))
        (i32.const 0 (; EXIT ;)))
      (func (export "run-cb") (param i32 i32 i32) (result i32) unreachable))
    (core instance $d (instantiate $d (with "" (instance
      (export "s" (func $s))
      (export "y" (func $y))
      (export "z" (func $z))
      (export "task.return" (func $task.return))))))
    (func (export "run") async (result u32)
      (canon lift (core func $d "run") async
        (callback (core func $d "run-cb")))))
  (instance $d (instantiate $D (with "x" (instance $x))))
  (export "run" (func $d "run"))
)

(assert_return (invoke "run") (u32.const 2 (; RETURNED ;)))

(component
  (component $F
    (core func $waitable-set.new (canon waitable-set.new))
    (core func $waitable-set.drop (canon waitable-set.drop))
    (core func $task.return (canon task.return))
    (core module $m
      (import "" "waitable-set.new" (func $waitable-set.new (result i32)))
      (import "" "waitable-set.drop" (func $waitable-set.drop (param i32)))
      (import "" "task.return" (func $task.return))
      (func (export "g") (result i32)
        (call $waitable-set.drop (call $waitable-set.new))
        (i32.const 7))
      (func (export "k") (result i32) (call $task.return) (i32.const 0))
      (func (export "cb") (param i32 i32 i32) (result i32) unreachable))
    (core instance $i (instantiate $m
      (with "" (instance
        (export "waitable-set.new" (func $waitable-set.new))
        (export "waitable-set.drop" (func $waitable-set.drop))
        (export "task.return" (func $task.return))))))
    (func (export "g") (result u32) (canon lift (core func $i "g")))
    (func (export "k") async
      (canon lift (core func $i "k") async
        (callback (core func $i "cb")))))
  (instance $f (instantiate $F))

  (component $E
    (import "f" (instance $f
      (export "g" (func (result u32)))
      (export "k" (func async))))
    (core func $g (canon lower (func $f "g")))
    (core func $k (canon lower (func $f "k") async))
    (core func $task.return (canon task.return))
    (core module $m
      (import "" "g" (func $g (result i32)))
      (import "" "k" (func $k (result i32)))
      (import "" "task.return" (func $task.return))
      (func (export "run") (result i32)
        (drop (call $g))
        (drop (call $k))
        (call $task.return)
        (i32.const 0))
      (func (export "cb") (param i32 i32 i32) (result i32) unreachable))
    (core instance $i (instantiate $m (with "" (instance
      (export "g" (func $g))
      (export "k" (func $k))
      (export "task.return" (func $task.return))))))
    (func (export "run") async
      (canon lift (core func $i "run") async (callback (core func $i "cb")))))
  (instance $e (instantiate $E (with "f" (instance $f))))

  (component $D
    (import "run" (func $run async))
    (core func $run (canon lower (func $run) async))
    (core func $task.return (canon task.return (result u32)))
    (core module $m
      (import "" "run" (func $run (result i32)))
      (import "" "task.return" (func $task.return (param i32)))
      (func (export "run") (result i32)
        (call $task.return (call $run))
        (i32.const 0))
      (func (export "cb") (param i32 i32 i32) (result i32) unreachable))
    (core instance $i (instantiate $m (with "" (instance
      (export "run" (func $run))
      (export "task.return" (func $task.return))))))
    (func (export "run") async (result u32)
      (canon lift (core func $i "run") async (callback (core func $i "cb")))))
  (instance $d (instantiate $D (with "run" (func $e "run"))))
  (export "run" (func $d "run"))
)

(assert_return (invoke "run") (u32.const 2))

(component
  (core module $shim
    (table (export "funcs") 1 1 funcref)
    (func (export "export") (result i32)
      (call_indirect (result i32) (i32.const 0))))
  (core instance $shim (instantiate $shim))
  (func $shim-export (result u32) (canon lift (core func $shim "export")))

  (component $X
    (import "import" (func $import (result u32)))
    (core func $import (canon lower (func $import)))
    (core func $waitable-set.new (canon waitable-set.new))
    (core func $waitable-set.drop (canon waitable-set.drop))
    (core module $m
      (import "" "import" (func $import (result i32)))
      (import "" "waitable-set.new" (func $waitable-set.new (result i32)))
      (import "" "waitable-set.drop" (func $waitable-set.drop (param i32)))
      (func (export "s") (result i32) (call $import))
      (func (export "h") (result i32)
        ;; any intrinsic that forces materialization of the deferred
        ;; sync->sync thread chain
        (call $waitable-set.drop (call $waitable-set.new))
        (i32.const 7)))
    (core instance $i (instantiate $m
      (with "" (instance
        (export "import" (func $import))
        (export "waitable-set.new" (func $waitable-set.new))
        (export "waitable-set.drop" (func $waitable-set.drop))))))
    (func (export "s") (result u32) (canon lift (core func $i "s")))
    (func (export "h") async (result u32) (canon lift (core func $i "h"))))
  (instance $x (instantiate $X (with "import" (func $shim-export))))

  (core func $x-h (canon lower (func $x "h")))
  (core module $donut
    (import "" "funcs" (table 1 1 funcref))
    (import "" "h" (func $h (result i32)))
    (func $guest-export (result i32) (call $h))
    (elem declare func $guest-export)
    (func $start (table.set (i32.const 0) (ref.func $guest-export)))
    (start $start))
  (core instance $donut (instantiate $donut
    (with "" (instance
      (export "h" (func $x-h))
      (export "funcs" (table $shim "funcs"))))))
  (export "s" (func $x "s"))
)

(assert_return (invoke "s") (u32.const 7))
