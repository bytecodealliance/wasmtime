;;! component_model_async = true
;;! component_model_more_async_builtins = true
;;! reference_types = true

;; Regression test for https://github.com/bytecodealliance/wasmtime/issues/13690.
;;
;; Per https://github.com/WebAssembly/component-model/pull/647, a *synchronous*
;; `{stream,future}.cancel-{read,write}` and `subtask.cancel` must trap when the
;; waitable being cancelled has already been added to a waitable set, for the
;; same reason the synchronous read/write operations do: the synchronous cancel
;; may need to block on the very waitable the set is concurrently watching.
;;
;; Each exported entry point is lifted `async` so that the synchronous operation
;; is permitted to reach the waitable-set check rather than tripping the
;; may-not-block check.

;; future.cancel-write
(component
  (type $f (future))

  (core module $libc (memory (export "mem") 1))
  (core instance $libc (instantiate $libc))

  (core func $new (canon future.new $f))
  (core func $write (canon future.write $f async (memory (core memory $libc "mem"))))
  (core func $cancel (canon future.cancel-write $f))
  (core func $ws-new (canon waitable-set.new))
  (core func $join (canon waitable.join))

  (core module $m
    (import "" "new" (func $new (result i64)))
    (import "" "write" (func $write (param i32 i32) (result i32)))
    (import "" "cancel" (func $cancel (param i32) (result i32)))
    (import "" "ws-new" (func $ws-new (result i32)))
    (import "" "join" (func $join (param i32 i32)))

    (func (export "f") (result i32)
      (local $write i32)
      (local $new i64)

      (local.set $new (call $new))
      (local.set $write (i32.wrap_i64 (i64.shr_u (local.get $new) (i64.const 32))))

      ;; start a write; with no reader present it blocks (BLOCKED == -1)
      (if (i32.ne (i32.const -1) (call $write (local.get $write) (i32.const 0)))
        (then unreachable))

      ;; add the writable end to a freshly created waitable set
      (call $join (local.get $write) (call $ws-new))

      ;; a synchronous cancel-write must now trap
      (call $cancel (local.get $write))
    )
  )

  (core instance $i (instantiate $m
    (with "" (instance
      (export "new" (func $new))
      (export "write" (func $write))
      (export "cancel" (func $cancel))
      (export "ws-new" (func $ws-new))
      (export "join" (func $join))
    ))
  ))

  (func (export "f") async (result u32) (canon lift (core func $i "f")))
)
(assert_trap (invoke "f") "waitable cannot be used synchronously while added to a waitable set")

;; future.cancel-read
(component
  (type $f (future))

  (core module $libc (memory (export "mem") 1))
  (core instance $libc (instantiate $libc))

  (core func $new (canon future.new $f))
  (core func $read (canon future.read $f async (memory (core memory $libc "mem"))))
  (core func $cancel (canon future.cancel-read $f))
  (core func $ws-new (canon waitable-set.new))
  (core func $join (canon waitable.join))

  (core module $m
    (import "" "new" (func $new (result i64)))
    (import "" "read" (func $read (param i32 i32) (result i32)))
    (import "" "cancel" (func $cancel (param i32) (result i32)))
    (import "" "ws-new" (func $ws-new (result i32)))
    (import "" "join" (func $join (param i32 i32)))

    (func (export "f") (result i32)
      (local $read i32)
      (local $new i64)

      (local.set $new (call $new))
      (local.set $read (i32.wrap_i64 (local.get $new)))

      ;; start a read; with no writer present it blocks (BLOCKED == -1)
      (if (i32.ne (i32.const -1) (call $read (local.get $read) (i32.const 0)))
        (then unreachable))

      ;; add the readable end to a freshly created waitable set
      (call $join (local.get $read) (call $ws-new))

      ;; a synchronous cancel-read must now trap
      (call $cancel (local.get $read))
    )
  )

  (core instance $i (instantiate $m
    (with "" (instance
      (export "new" (func $new))
      (export "read" (func $read))
      (export "cancel" (func $cancel))
      (export "ws-new" (func $ws-new))
      (export "join" (func $join))
    ))
  ))

  (func (export "f") async (result u32) (canon lift (core func $i "f")))
)
(assert_trap (invoke "f") "waitable cannot be used synchronously while added to a waitable set")

;; stream.cancel-write
(component
  (type $s (stream u8))

  (core module $libc (memory (export "mem") 1))
  (core instance $libc (instantiate $libc))

  (core func $new (canon stream.new $s))
  (core func $write (canon stream.write $s async (memory (core memory $libc "mem"))))
  (core func $cancel (canon stream.cancel-write $s))
  (core func $ws-new (canon waitable-set.new))
  (core func $join (canon waitable.join))

  (core module $m
    (import "" "new" (func $new (result i64)))
    (import "" "write" (func $write (param i32 i32 i32) (result i32)))
    (import "" "cancel" (func $cancel (param i32) (result i32)))
    (import "" "ws-new" (func $ws-new (result i32)))
    (import "" "join" (func $join (param i32 i32)))

    (func (export "f") (result i32)
      (local $write i32)
      (local $new i64)

      (local.set $new (call $new))
      (local.set $write (i32.wrap_i64 (i64.shr_u (local.get $new) (i64.const 32))))

      ;; start a write; with no reader present it blocks (BLOCKED == -1)
      (if (i32.ne (i32.const -1) (call $write (local.get $write) (i32.const 0) (i32.const 4)))
        (then unreachable))

      ;; add the writable end to a freshly created waitable set
      (call $join (local.get $write) (call $ws-new))

      ;; a synchronous cancel-write must now trap
      (call $cancel (local.get $write))
    )
  )

  (core instance $i (instantiate $m
    (with "" (instance
      (export "new" (func $new))
      (export "write" (func $write))
      (export "cancel" (func $cancel))
      (export "ws-new" (func $ws-new))
      (export "join" (func $join))
    ))
  ))

  (func (export "f") async (result u32) (canon lift (core func $i "f")))
)
(assert_trap (invoke "f") "waitable cannot be used synchronously while added to a waitable set")

;; stream.cancel-read
(component
  (type $s (stream u8))

  (core module $libc (memory (export "mem") 1))
  (core instance $libc (instantiate $libc))

  (core func $new (canon stream.new $s))
  (core func $read (canon stream.read $s async (memory (core memory $libc "mem"))))
  (core func $cancel (canon stream.cancel-read $s))
  (core func $ws-new (canon waitable-set.new))
  (core func $join (canon waitable.join))

  (core module $m
    (import "" "new" (func $new (result i64)))
    (import "" "read" (func $read (param i32 i32 i32) (result i32)))
    (import "" "cancel" (func $cancel (param i32) (result i32)))
    (import "" "ws-new" (func $ws-new (result i32)))
    (import "" "join" (func $join (param i32 i32)))

    (func (export "f") (result i32)
      (local $read i32)
      (local $new i64)

      (local.set $new (call $new))
      (local.set $read (i32.wrap_i64 (local.get $new)))

      ;; start a read; with no writer present it blocks (BLOCKED == -1)
      (if (i32.ne (i32.const -1) (call $read (local.get $read) (i32.const 0) (i32.const 4)))
        (then unreachable))

      ;; add the readable end to a freshly created waitable set
      (call $join (local.get $read) (call $ws-new))

      ;; a synchronous cancel-read must now trap
      (call $cancel (local.get $read))
    )
  )

  (core instance $i (instantiate $m
    (with "" (instance
      (export "new" (func $new))
      (export "read" (func $read))
      (export "cancel" (func $cancel))
      (export "ws-new" (func $ws-new))
      (export "join" (func $join))
    ))
  ))

  (func (export "f") async (result u32) (canon lift (core func $i "f")))
)
(assert_trap (invoke "f") "waitable cannot be used synchronously while added to a waitable set")

;; subtask.cancel
(component
  ;; A callee whose async export applies backpressure so the caller's subtask
  ;; stays pending (and is therefore a live waitable that can join a set).
  (component $callee
    (core module $m
      (import "" "backpressure.inc" (func $backpressure.inc))
      (func (export "set-backpressure") (call $backpressure.inc))
      (func (export "block"))
    )
    (core func $backpressure.inc (canon backpressure.inc))
    (core instance $i (instantiate $m
      (with "" (instance (export "backpressure.inc" (func $backpressure.inc))))
    ))
    (func (export "set-backpressure") (canon lift (core func $i "set-backpressure")))
    (func (export "block") async (canon lift (core func $i "block")))
  )
  (instance $callee (instantiate $callee))

  (component $caller
    (import "callee" (instance $callee
      (export "set-backpressure" (func))
      (export "block" (func async))
    ))

    (core func $set-backpressure (canon lower (func $callee "set-backpressure")))
    (core func $block (canon lower (func $callee "block") async))
    (core func $cancel (canon subtask.cancel))
    (core func $ws-new (canon waitable-set.new))
    (core func $join (canon waitable.join))

    (core module $m
      (import "" "set-backpressure" (func $set-backpressure))
      (import "" "block" (func $block (result i32)))
      (import "" "cancel" (func $cancel (param i32) (result i32)))
      (import "" "ws-new" (func $ws-new (result i32)))
      (import "" "join" (func $join (param i32 i32)))

      (func (export "f")
        (local $rc i32)
        (local $task i32)

        ;; ensure the callee can't complete eagerly
        (call $set-backpressure)

        ;; start the subtask; it stays pending under backpressure
        (local.set $rc (call $block))
        (local.set $task (i32.shr_u (local.get $rc) (i32.const 4)))

        ;; add the subtask to a freshly created waitable set
        (call $join (local.get $task) (call $ws-new))

        ;; a synchronous subtask.cancel must now trap
        (drop (call $cancel (local.get $task)))
      )
    )

    (core instance $i (instantiate $m
      (with "" (instance
        (export "set-backpressure" (func $set-backpressure))
        (export "block" (func $block))
        (export "cancel" (func $cancel))
        (export "ws-new" (func $ws-new))
        (export "join" (func $join))
      ))
    ))

    (func (export "f") async (canon lift (core func $i "f")))
  )
  (instance $caller (instantiate $caller (with "callee" (instance $callee))))
  (func (export "f") (alias export $caller "f"))
)
(assert_trap (invoke "f") "waitable cannot be used synchronously while added to a waitable set")
