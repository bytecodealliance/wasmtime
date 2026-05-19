;;! component_model_async = true
;;! component_model_more_async_builtins = true
;;! reference_types = true

(component
  (type $FT (future u8))
  (core module $Memory (memory (export "mem") 1))

  (component $C
    (core instance $memory (instantiate $Memory))
    (core module $CM
      (import "" "mem" (memory 1))
      (import "" "waitable.join" (func $waitable.join (param i32 i32)))
      (import "" "waitable-set.new" (func $waitable-set.new (result i32)))
      (import "" "waitable-set.wait" (func $waitable-set.wait (param i32 i32) (result i32)))
      (import "" "future.new" (func $future.new (result i64)))
      (import "" "future.write-sync" (func $future.write-sync (param i32 i32) (result i32)))

      (global $writable-end (mut i32) (i32.const 0))
      (global $ws (mut i32) (i32.const 0))

      ;; create a new future, return the readable end to the caller
      (func $start-future (export "start-future") (result i32)
        (local $ret64 i64)
        (global.set $ws (call $waitable-set.new))
        (local.set $ret64 (call $future.new))
        (global.set $writable-end (i32.wrap_i64 (i64.shr_u (local.get $ret64) (i64.const 32))))
        (call $waitable.join (global.get $writable-end) (global.get $ws) )
        (i32.wrap_i64 (local.get $ret64))
      )
      (func $future-write-sync (export "future-write-sync") (result i32)
        ;; the caller will assert what they expect the return value to be
        (i32.store (i32.const 16) (i32.const 42))
        (call $future.write-sync (global.get $writable-end) (i32.const 16))
      )
      (func $acknowledge-future-write (export "acknowledge-future-write")
        ;; confirm we got a FUTURE_WRITE $writable-end COMPLETED event
        (local $ret i32)
        (local.set $ret (call $waitable-set.wait (global.get $ws) (i32.const 0)))

        ;; This should trap per https://github.com/WebAssembly/component-model/pull/647
        (if (i32.ne (i32.const 5 (; FUTURE_WRITE ;)) (local.get $ret))
          (then unreachable))
        (if (i32.ne (global.get $writable-end) (i32.load (i32.const 0)))
          (then unreachable))
        (if (i32.ne (i32.const 0 (; COMPLETED ;)) (i32.load (i32.const 4)))
          (then unreachable))
      )
    )
    (canon waitable.join (core func $waitable.join))
    (canon waitable-set.new (core func $waitable-set.new))
    (canon waitable-set.wait (memory $memory "mem") (core func $waitable-set.wait))
    (canon future.new $FT (core func $future.new))
    (canon future.write $FT (memory $memory "mem") (core func $future.write-sync))
    (core instance $cm (instantiate $CM (with "" (instance
      (export "mem" (memory $memory "mem"))
      (export "waitable.join" (func $waitable.join))
      (export "waitable-set.new" (func $waitable-set.new))
      (export "waitable-set.wait" (func $waitable-set.wait))
      (export "future.new" (func $future.new))
      (export "future.write-sync" (func $future.write-sync))
    ))))
    (func (export "start-future") (result (future u8)) (canon lift (core func $cm "start-future")))
    (func (export "future-write-sync") async (result u32) (canon lift (core func $cm "future-write-sync")))
    (func (export "acknowledge-future-write") async (canon lift (core func $cm "acknowledge-future-write")))
  )

  (component $D
    (import "c" (instance $c
      (export "start-future" (func (result (future u8))))
      (export "future-write-sync" (func async (result u32)))
      (export "acknowledge-future-write" (func async))
    ))

    (core instance $memory (instantiate $Memory))
    (core module $Core
      (import "" "mem" (memory 1))
      (import "" "future.read" (func $future.read (param i32 i32) (result i32)))
      (import "" "start-future" (func $start-future (result i32)))
      (import "" "future-write-sync.async" (func $future-write-sync.async (param i32) (result i32)))
      (import "" "acknowledge-future-write" (func $acknowledge-future-write))

      (func $trap-after-future-async-write (export "trap-after-future-async-write")
        (local $ret i32)
        (local $fr i32)
        (local $subtask i32)
        (local.set $fr (call $start-future))

        ;; calling future.write in $C should block
        (local.set $ret (call $future-write-sync.async (i32.const 4)))
        (if (i32.ne (i32.const 1 (; SUBTASK_STARTED ;)) (i32.and (local.get $ret) (i32.const 0xf)))
          (then unreachable))

        ;; our future.read should then succeed eagerly
        (local.set $ret (call $future.read (local.get $fr) (i32.const 16)))
        (if (i32.ne (i32.const 0 (; COMPLETED ;)) (local.get $ret))
          (then unreachable))
        (if (i32.ne (i32.const 42) (i32.load8_u (i32.const 16)))
          (then unreachable))

        ;; try to use a waitable-set to acquire an event...
        (call $acknowledge-future-write)
      )
    )
    (canon future.read $FT async (memory $memory "mem") (core func $future.read))
    (canon lower (func $c "start-future") (core func $start-future'))
    (canon lower (func $c "future-write-sync") async (memory $memory "mem") (core func $future-write-sync.async))
    (canon lower (func $c "acknowledge-future-write") (core func $acknowledge-future-write'))
    (core instance $core (instantiate $Core (with "" (instance
      (export "mem" (memory $memory "mem"))
      (export "future.read" (func $future.read))
      (export "start-future" (func $start-future'))
      (export "future-write-sync.async" (func $future-write-sync.async))
      (export "acknowledge-future-write" (func $acknowledge-future-write'))
    ))))
    (func (export "trap-after-future-async-write") async (canon lift (core func $core "trap-after-future-async-write")))
  )
  (instance $c (instantiate $C))
  (instance $d (instantiate $D (with "c" (instance $c))))
  (func (export "trap-after-future-async-write") (alias export $d "trap-after-future-async-write"))
)

(assert_trap (invoke "trap-after-future-async-write") "waitable cannot be used synchronously while added to a waitable set")
