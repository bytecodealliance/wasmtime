;;! component_model_async = true
;;! component_model_async_stackful = true
;;! reference_types = true

;; This is similar to the `drop-waitable-set.wast` test except that it uses
;; "stackful" (i.e. no callback) async-lifted exports instead of "stackless"
;; (i.e. with a callback) exports.  That creates a situation where the waiter on
;; the waitable set being dropped is a suspended fiber.  Historically, there was
;; a bug in Wasmtime such that we checked for waiters _after_ removing the set
;; from the table, causing the fiber to be dropped and leading to a panic due to
;; the fiber not having been disposed of gracefully.

(component
  (component $C
    (core module $Memory (memory (export "mem") 1))
    (core instance $memory (instantiate $Memory))
    (core module $Core
      (import "" "mem" (memory 1))
      (import "" "waitable-set.new" (func $waitable-set.new (result i32)))
      (import "" "waitable-set.wait" (func $waitable-set.wait (param i32 i32) (result i32)))
      (import "" "waitable-set.drop" (func $waitable-set.drop (param i32)))

      (global $ws (mut i32) (i32.const 0))
      (func $start (global.set $ws (call $waitable-set.new)))
      (start $start)

      ;; Stackful async: calls waitable-set.wait directly (blocks the fiber).
      ;; The set is empty, so this will block indefinitely.
      (func $wait-on-set (export "wait-on-set")
        (drop (call $waitable-set.wait (global.get $ws) (i32.const 0)))
      )

      ;; Attempts to drop the set while a fiber is waiting on it.
      (func $drop-while-waiting (export "drop-while-waiting")
        (call $waitable-set.drop (global.get $ws))
        unreachable
      )
    )
    (canon waitable-set.new (core func $waitable-set.new))
    (canon waitable-set.wait (memory $memory "mem") (core func $waitable-set.wait))
    (canon waitable-set.drop (core func $waitable-set.drop))
    (core instance $core (instantiate $Core (with "" (instance
      (export "mem" (memory $memory "mem"))
      (export "waitable-set.new" (func $waitable-set.new))
      (export "waitable-set.wait" (func $waitable-set.wait))
      (export "waitable-set.drop" (func $waitable-set.drop))
    ))))

    ;; KEY DIFFERENCE from callback test: `async` without `(callback ...)`.
    ;; This makes the export use stackful fiber mode instead of callback mode.
    ;; The core function runs on a fiber and can call blocking builtins directly.
    (func (export "wait-on-set") async (canon lift
      (core func $core "wait-on-set")
      async
    ))
    (func (export "drop-while-waiting") async (canon lift
      (core func $core "drop-while-waiting")
      async
    ))
  )

  (component $D
    (import "c" (instance $c
      (export "wait-on-set" (func async))
      (export "drop-while-waiting" (func async))
    ))

    (core module $Memory (memory (export "mem") 1))
    (core instance $memory (instantiate $Memory))
    (core module $Core
      (import "" "mem" (memory 1))
      (import "" "wait-on-set" (func $wait-on-set (result i32)))
      (import "" "drop-while-waiting" (func $drop-while-waiting))
      (func $run (export "run") (result i32)
        (local $ret i32)

        ;; Start an async call to wait-on-set. The callee's core function
        ;; runs on a fiber and calls waitable-set.wait, which suspends it.
        ;; The return value encodes (subtask_handle << 4) | status.
        (local.set $ret (call $wait-on-set))
        (if (i32.ne (i32.const 1 (; STARTED ;)) (i32.and (local.get $ret) (i32.const 0xf)))
          (then unreachable))

        ;; Now call drop-while-waiting, which tries to drop the waitable set
        ;; that has a suspended fiber waiting on it.
        (call $drop-while-waiting)
        unreachable
      )
    )
    (canon lower (func $c "wait-on-set") async (memory $memory "mem") (core func $wait-on-set'))
    (canon lower (func $c "drop-while-waiting") (core func $drop-while-waiting'))
    (core instance $core (instantiate $Core (with "" (instance
      (export "mem" (memory $memory "mem"))
      (export "wait-on-set" (func $wait-on-set'))
      (export "drop-while-waiting" (func $drop-while-waiting'))
    ))))
    (func (export "run") async (result u32) (canon lift (core func $core "run")))
  )

  (instance $c (instantiate $C))
  (instance $d (instantiate $D (with "c" (instance $c))))
  (func (export "run") (alias export $d "run"))
)

(assert_trap (invoke "run") "cannot drop waitable set with waiters")
