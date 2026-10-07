;;! component_model_async = true
;;! reference_types = true
;;! bulk_memory = true

;; A sync-lowered call to a sync-lifted export with an `async` function type is
;; subject to the same backpressure and exclusive-entry checks as any other
;; call to an `async`-typed function (see `Task.enter_implicit_thread` in the
;; spec): the callee doesn't start until both allow it, and the sync-lowered
;; caller blocks in the meantime.

;; Backpressure is set on the callee's instance and never released, so the
;; callee never starts and the caller blocks forever.
(component
  (component $X
    (core func $backpressure.inc (canon backpressure.inc))
    (core module $x
      (import "" "backpressure.inc" (func $backpressure.inc))
      (func (export "bp-inc") (call $backpressure.inc))
      (func (export "h") (result i32) unreachable))
    (core instance $x (instantiate $x
      (with "" (instance
        (export "backpressure.inc" (func $backpressure.inc))))))
    (func (export "bp-inc") (canon lift (core func $x "bp-inc")))
    (func (export "h") async (result u32) (canon lift (core func $x "h")))
  )
  (component $Y
    (import "x" (instance $x
      (export "bp-inc" (func))
      (export "h" (func async (result u32)))))
    (core func $bp-inc (canon lower (func $x "bp-inc")))
    (core func $h (canon lower (func $x "h")))
    (core func $task.return (canon task.return (result u32)))
    (core module $y
      (import "" "bp-inc" (func $bp-inc))
      (import "" "h" (func $h (result i32)))
      (import "" "task.return" (func $task.return (param i32)))
      (func (export "run") (result i32)
        (call $bp-inc)
        call $h
        unreachable
      )
      (func (export "callback") (param i32 i32 i32) (result i32) unreachable))
    (core instance $y (instantiate $y (with "" (instance
      (export "bp-inc" (func $bp-inc))
      (export "h" (func $h))
      (export "task.return" (func $task.return))))))
    (func (export "run") async (result u32)
      (canon lift (core func $y "run") async
        (callback (core func $y "callback"))))
  )
  (instance $x (instantiate $X))
  (instance $y (instantiate $Y (with "x" (instance $x))))
  (export "run" (func $y "run"))
)

(assert_trap (invoke "run") "deadlock detected")

;; Backpressure is set on the callee's instance and later released by another
;; task in that instance, at which point the callee starts and the blocked
;; caller resumes.
(component
  (component $X
    (core func $backpressure.inc (canon backpressure.inc))
    (core func $backpressure.dec (canon backpressure.dec))
    (core func $task.return (canon task.return))
    (core module $x
      (import "" "backpressure.inc" (func $backpressure.inc))
      (import "" "backpressure.dec" (func $backpressure.dec))
      (import "" "task.return" (func $task.return))
      (global $released (mut i32) (i32.const 0))
      (func (export "bp-inc") (call $backpressure.inc))
      (func (export "release") (result i32)
        i32.const 1 ;; YIELD
      )
      (func (export "release-cb") (param i32 i32 i32) (result i32)
        (global.set $released (i32.const 1))
        (call $backpressure.dec)
        (call $task.return)
        i32.const 0 ;; EXIT
      )
      (func (export "h") (result i32)
        ;; `h` must not start until backpressure has been released.
        (if (i32.eqz (global.get $released)) (then unreachable))
        i32.const 42))
    (core instance $x (instantiate $x (with "" (instance
      (export "backpressure.inc" (func $backpressure.inc))
      (export "backpressure.dec" (func $backpressure.dec))
      (export "task.return" (func $task.return))))))
    (func (export "bp-inc") (canon lift (core func $x "bp-inc")))
    (func (export "release") async
      (canon lift (core func $x "release") async
        (callback (core func $x "release-cb"))))
    (func (export "h") async (result u32) (canon lift (core func $x "h")))
  )
  (component $Y
    (import "x" (instance $x
      (export "bp-inc" (func))
      (export "release" (func async))
      (export "h" (func async (result u32)))))
    (core func $bp-inc (canon lower (func $x "bp-inc")))
    (core func $release (canon lower (func $x "release") async))
    (core func $h (canon lower (func $x "h")))
    (core func $task.return (canon task.return (result u32)))
    (core module $y
      (import "" "bp-inc" (func $bp-inc))
      (import "" "release" (func $release (result i32)))
      (import "" "h" (func $h (result i32)))
      (import "" "task.return" (func $task.return (param i32)))
      (func (export "run") (result i32)
        ;; Start a task in `x` which will release backpressure once it's
        ;; resumed after yielding.
        (drop (call $release))
        (call $bp-inc)
        ;; Blocks until `release` has released backpressure.
        (call $task.return (call $h))
        i32.const 0 ;; EXIT
      )
      (func (export "callback") (param i32 i32 i32) (result i32) unreachable))
    (core instance $y (instantiate $y (with "" (instance
      (export "bp-inc" (func $bp-inc))
      (export "release" (func $release))
      (export "h" (func $h))
      (export "task.return" (func $task.return))))))
    (func (export "run") async (result u32)
      (canon lift (core func $y "run") async
        (callback (core func $y "callback"))))
  )
  (instance $x (instantiate $X))
  (instance $y (instantiate $Y (with "x" (instance $x))))
  (export "run" (func $y "run"))
)

(assert_return (invoke "run") (u32.const 42))

;; `c`, a callback-lifted export of `$inner`, holds `$inner`'s exclusive lock
;; while it makes a sync-lowered call which, via an `async`-typed shim in the
;; outer component, makes a sync-lowered call to `h`, an `async`-typed but
;; sync-lifted export of `$inner`. `h` needs the exclusive lock which `c` never
;; releases, so it never starts.
(component
  (core module $shim
    (table (export "funcs") 1 1 funcref)
    (func (export "export") (result i32)
      (call_indirect (result i32) (i32.const 0))))
  (core instance $shim (instantiate $shim))
  (func $shim-export async (result u32) (canon lift (core func $shim "export")))

  (component $inner
    (import "import" (func $import async (result u32)))
    (core func $import (canon lower (func $import)))
    (core func $task.return (canon task.return (result u32)))
    (core module $m
      (import "" "import" (func $import (result i32)))
      (import "" "task.return" (func $task.return (param i32)))
      (func (export "c") (result i32)
        (call $task.return (call $import))
        i32.const 0 ;; EXIT
      )
      (func (export "callback") (param i32 i32 i32) (result i32) unreachable)
      (func (export "h") (result i32) (i32.const 7)))
    (core instance $i (instantiate $m
      (with "" (instance
        (export "task.return" (func $task.return))
        (export "import" (func $import))))))
    (func (export "c") async (result u32)
      (canon lift (core func $i "c") async
        (callback (core func $i "callback"))))
    (func (export "h") async (result u32)
      (canon lift (core func $i "h"))))
  (instance $inner (instantiate $inner (with "import" (func $shim-export))))

  (core func $inner-h (canon lower (func $inner "h")))
  (core module $donut
    (import "" "funcs" (table 1 1 funcref))
    (import "" "h" (func $h (result i32)))
    (func $guest-export (result i32) (call $h))
    (elem declare func $guest-export)
    (func $start (table.set (i32.const 0) (ref.func $guest-export)))
    (start $start))
  (core instance $donut (instantiate $donut
    (with "" (instance
      (export "h" (func $inner-h))
      (export "funcs" (table $shim "funcs"))))))
  (export "c" (func $inner "c"))
)

(assert_trap (invoke "c") "deadlock detected")

;; Same as above, but the shim is sync-typed. A sync-typed task which blocks
;; with no other thread in its instance to run traps.
(component
  (core module $shim
    (table (export "funcs") 1 1 funcref)
    (func (export "export") (result i32)
      (call_indirect (result i32) (i32.const 0))))
  (core instance $shim (instantiate $shim))
  (func $shim-export (result u32) (canon lift (core func $shim "export")))

  (component $inner
    (import "import" (func $import (result u32)))
    (core func $import (canon lower (func $import)))
    (core func $task.return (canon task.return (result u32)))
    (core module $m
      (import "" "import" (func $import (result i32)))
      (import "" "task.return" (func $task.return (param i32)))
      (func (export "c") (result i32)
        (call $task.return (call $import))
        i32.const 0 ;; EXIT
      )
      (func (export "callback") (param i32 i32 i32) (result i32) unreachable)
      (func (export "h") (result i32) (i32.const 7)))
    (core instance $i (instantiate $m
      (with "" (instance
        (export "task.return" (func $task.return))
        (export "import" (func $import))))))
    (func (export "c") async (result u32)
      (canon lift (core func $i "c") async
        (callback (core func $i "callback"))))
    (func (export "h") async (result u32)
      (canon lift (core func $i "h"))))
  (instance $inner (instantiate $inner (with "import" (func $shim-export))))

  (core func $inner-h (canon lower (func $inner "h")))
  (core module $donut
    (import "" "funcs" (table 1 1 funcref))
    (import "" "h" (func $h (result i32)))
    (func $guest-export (result i32) (call $h))
    (elem declare func $guest-export)
    (func $start (table.set (i32.const 0) (ref.func $guest-export)))
    (start $start))
  (core instance $donut (instantiate $donut
    (with "" (instance
      (export "h" (func $inner-h))
      (export "funcs" (table $shim "funcs"))))))
  (export "c" (func $inner "c"))
)

(assert_trap (invoke "c") "cannot block a synchronous task before returning")

;; `c` makes an async-lowered call to the `async`-typed shim, which in turn
;; makes a sync-lowered call to `h`. `h` can't start until `c` releases
;; `$inner`'s exclusive lock by returning `WAIT`, after which `h` runs and `c`
;; receives the result.
(component
  (core module $shim
    (table (export "funcs") 1 1 funcref)
    (func (export "export") (result i32)
      (call_indirect (result i32) (i32.const 0))))
  (core instance $shim (instantiate $shim))
  (func $shim-export async (result u32) (canon lift (core func $shim "export")))

  (component $inner
    (import "import" (func $import async (result u32)))
    (core module $libc (memory (export "mem") 1))
    (core instance $libc (instantiate $libc))
    (core func $import
      (canon lower (func $import) async (memory (core memory $libc "mem"))))
    (core func $task.return (canon task.return (result u32)))
    (core func $waitable-set.new (canon waitable-set.new))
    (core func $waitable-set.drop (canon waitable-set.drop))
    (core func $waitable.join (canon waitable.join))
    (core func $subtask.drop (canon subtask.drop))
    (core module $m
      (import "" "mem" (memory 1))
      (import "" "import" (func $import (param i32) (result i32)))
      (import "" "task.return" (func $task.return (param i32)))
      (import "" "waitable-set.new" (func $waitable-set.new (result i32)))
      (import "" "waitable-set.drop" (func $waitable-set.drop (param i32)))
      (import "" "waitable.join" (func $waitable.join (param i32 i32)))
      (import "" "subtask.drop" (func $subtask.drop (param i32)))
      (global $in-c (mut i32) (i32.const 0))
      (global $set (mut i32) (i32.const 0))
      (func (export "c") (result i32)
        (local $status i32)
        (global.set $in-c (i32.const 1))
        (local.set $status (call $import (i32.const 100)))
        ;; The subtask must not have returned yet since `h` can't start until
        ;; we return.
        (if (i32.ne (i32.and (local.get $status) (i32.const 0xf)) (i32.const 1))
          (then unreachable))
        (global.set $set (call $waitable-set.new))
        (call $waitable.join
          (i32.shr_u (local.get $status) (i32.const 4))
          (global.get $set))
        (global.set $in-c (i32.const 0))
        ;; WAIT on `$set`
        (i32.or (i32.const 2) (i32.shl (global.get $set) (i32.const 4)))
      )
      (func (export "callback")
        (param $event i32) (param $handle i32) (param $status i32) (result i32)
        ;; expect EVENT_SUBTASK with STATUS_RETURNED
        (if (i32.ne (local.get $event) (i32.const 1)) (then unreachable))
        (if (i32.ne (local.get $status) (i32.const 2)) (then unreachable))
        (call $subtask.drop (local.get $handle))
        (call $waitable-set.drop (global.get $set))
        (call $task.return (i32.load (i32.const 100)))
        i32.const 0 ;; EXIT
      )
      (func (export "h") (result i32)
        ;; `h` must not run while `c` is running.
        (if (global.get $in-c) (then unreachable))
        i32.const 7))
    (core instance $i (instantiate $m
      (with "" (instance
        (export "mem" (memory $libc "mem"))
        (export "task.return" (func $task.return))
        (export "import" (func $import))
        (export "waitable-set.new" (func $waitable-set.new))
        (export "waitable-set.drop" (func $waitable-set.drop))
        (export "waitable.join" (func $waitable.join))
        (export "subtask.drop" (func $subtask.drop))))))
    (func (export "c") async (result u32)
      (canon lift (core func $i "c") async
        (callback (core func $i "callback"))))
    (func (export "h") async (result u32)
      (canon lift (core func $i "h"))))
  (instance $inner (instantiate $inner (with "import" (func $shim-export))))

  (core func $inner-h (canon lower (func $inner "h")))
  (core module $donut
    (import "" "funcs" (table 1 1 funcref))
    (import "" "h" (func $h (result i32)))
    (func $guest-export (result i32) (call $h))
    (elem declare func $guest-export)
    (func $start (table.set (i32.const 0) (ref.func $guest-export)))
    (start $start))
  (core instance $donut (instantiate $donut
    (with "" (instance
      (export "h" (func $inner-h))
      (export "funcs" (table $shim "funcs"))))))
  (export "c" (func $inner "c"))
)

(assert_return (invoke "c") (u32.const 7))
