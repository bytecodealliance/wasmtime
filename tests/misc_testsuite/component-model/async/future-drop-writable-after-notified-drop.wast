;;! component_model_async = true
;;! reference_types = true

(component
  ;; The writer.
  (component $A
    (core module $libc (memory (export "mem") 1))
    (core instance $libc (instantiate $libc))

    (type $f (future u32))
    (core func $future.new (canon future.new $f))
    (core func $future.write (canon future.write $f async (memory (core memory $libc "mem"))))
    (core func $future.drop-writable (canon future.drop-writable $f))
    (core func $task.return (canon task.return))
    (core func $waitable-set.new (canon waitable-set.new))
    (core func $waitable-set.drop (canon waitable-set.drop))
    (core func $waitable.join (canon waitable.join))

    (core module $cm
      (import "" "future.new" (func $future.new (result i64)))
      (import "" "future.write" (func $future.write (param i32 i32) (result i32)))
      (import "" "future.drop-writable" (func $future.drop-writable (param i32)))
      (import "" "task.return" (func $task.return))
      (import "" "waitable-set.new" (func $waitable-set.new (result i32)))
      (import "" "waitable-set.drop" (func $waitable-set.drop (param i32)))
      (import "" "waitable.join" (func $waitable.join (param i32 i32)))

      (global $writer (mut i32) (i32.const 0))
      (global $set (mut i32) (i32.const 0))

      (func (export "get-future") (result i32)
        (local $pair i64)
        (local.set $pair (call $future.new))
        (global.set $writer (i32.wrap_i64 (i64.shr_u (local.get $pair) (i64.const 32))))
        (i32.wrap_i64 (local.get $pair))
      )

      (func (export "write-and-wait") (result i32)
        (call $future.write (global.get $writer) (i32.const 0))
        i32.const -1 ;; BLOCKED
        i32.ne
        if unreachable end

        (global.set $set (call $waitable-set.new))
        (call $waitable.join (global.get $writer) (global.get $set))
        (i32.or
          (i32.const 2) ;; CALLBACK_CODE_WAIT
          (i32.shl (global.get $set) (i32.const 4)))
      )

      (func (export "write-and-wait-cb") (param $event i32) (param $waitable i32) (param $code i32) (result i32)
        (if (i32.ne (local.get $event) (i32.const 5)) ;; EVENT_FUTURE_WRITE
          (then unreachable))
        (if (i32.ne (local.get $waitable) (global.get $writer))
          (then unreachable))
        (if (i32.ne (local.get $code) (i32.const 1)) ;; DROPPED
          (then unreachable))

        ;; Clean up everything.
        (call $waitable.join (local.get $waitable) (i32.const 0))
        (call $waitable-set.drop (global.get $set))
        (call $future.drop-writable (local.get $waitable))
        (call $task.return)
        (i32.const 0) ;; CALLBACK_CODE_EXIT
      )
    )
    (core instance $ci (instantiate $cm (with "" (instance
      (export "future.new" (func $future.new))
      (export "future.write" (func $future.write))
      (export "future.drop-writable" (func $future.drop-writable))
      (export "task.return" (func $task.return))
      (export "waitable-set.new" (func $waitable-set.new))
      (export "waitable-set.drop" (func $waitable-set.drop))
      (export "waitable.join" (func $waitable.join))
    ))))

    (func (export "get-future") (result (future u32))
      (canon lift (core func $ci "get-future")))
    (func (export "write-and-wait") async
      (canon lift (core func $ci "write-and-wait") async (callback (core func $ci "write-and-wait-cb"))))
  )

  (component $B
    (import "a" (instance $a
      (export "get-future" (func (result (future u32))))
      (export "write-and-wait" (func async))
    ))

    (core module $libc (memory (export "mem") 1))
    (core instance $libc (instantiate $libc))

    (type $f (future u32))
    (core func $future.drop-readable (canon future.drop-readable $f))
    (core func $get-future (canon lower (func $a "get-future")))
    (core func $write-and-wait (canon lower (func $a "write-and-wait") async))
    (core func $waitable-set.new (canon waitable-set.new))
    (core func $waitable-set.drop (canon waitable-set.drop))
    (core func $waitable.join (canon waitable.join))
    (core func $waitable-set.wait (canon waitable-set.wait (memory (core memory $libc "mem"))))
    (core func $subtask.drop (canon subtask.drop))

    (core module $dm
      (import "" "future.drop-readable" (func $future.drop-readable (param i32)))
      (import "" "get-future" (func $get-future (result i32)))
      (import "" "write-and-wait" (func $write-and-wait (result i32)))
      (import "" "waitable-set.new" (func $waitable-set.new (result i32)))
      (import "" "waitable-set.drop" (func $waitable-set.drop (param i32)))
      (import "" "waitable.join" (func $waitable.join (param i32 i32)))
      (import "" "waitable-set.wait" (func $waitable-set.wait (param i32 i32) (result i32)))
      (import "" "subtask.drop" (func $subtask.drop (param i32)))

      (func (export "run")
        (local $reader i32)
        (local $status i32)
        (local $subtask i32)
        (local $ws i32)

        ;; Acquire the readable end from $C.
        (local.set $reader (call $get-future))

        ;; Start $C's async writer; it blocks on the write and reports STARTED.
        (local.set $status (call $write-and-wait))
        (if (i32.ne (i32.and (local.get $status) (i32.const 0xf)) (i32.const 1)) ;; STARTED
          (then unreachable))
        (local.set $subtask (i32.shr_u (local.get $status) (i32.const 4)))

        ;; Drop the readable end without reading.  This notifies $C's pending
        ;; write that the reader is gone.
        (call $future.drop-readable (local.get $reader))

        ;; Wait for $C's subtask to finish
        (local.set $ws (call $waitable-set.new))
        (call $waitable.join (local.get $subtask) (local.get $ws))
        (drop (call $waitable-set.wait (local.get $ws) (i32.const 0)))

        (call $subtask.drop (local.get $subtask))
        (call $waitable-set.drop (local.get $ws))
      )
    )
    (core instance $di (instantiate $dm (with "" (instance
      (export "future.drop-readable" (func $future.drop-readable))
      (export "get-future" (func $get-future))
      (export "write-and-wait" (func $write-and-wait))
      (export "waitable-set.new" (func $waitable-set.new))
      (export "waitable-set.drop" (func $waitable-set.drop))
      (export "waitable.join" (func $waitable.join))
      (export "waitable-set.wait" (func $waitable-set.wait))
      (export "subtask.drop" (func $subtask.drop))
    ))))

    (func (export "run") async
      (canon lift (core func $di "run")))
  )

  (instance $a (instantiate $A))
  (instance $b (instantiate $B (with "a" (instance $a))))
  (func (export "run") (alias export $b "run"))
)

(assert_return (invoke "run"))
