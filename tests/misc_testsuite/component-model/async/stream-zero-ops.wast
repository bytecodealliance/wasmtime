;;! component_model_async = true
;;! reference_types = true

;; This test asserts some behavior with 0-length reads/writes rendezvous in the
;; component model. Specifically when a writer meets a waiting reader it doesn't
;; unblock the reader and the writer keeps going.
(component
  (type $ST (stream))

  (core module $libc (memory (export "m") 1))

  (component $A
    (core module $m
      (import "libc" "m" (memory 1))
      (import "" "stream.read" (func $stream.read (param i32 i32 i32) (result i32)))
      (import "" "stream.drop-readable" (func $stream.drop-readable (param i32)))
      (import "" "waitable-set.new" (func $waitable-set.new (result i32)))
      (import "" "waitable.join" (func $waitable.join (param i32 i32)))
      (import "" "waitable-set.drop" (func $waitable-set.drop (param i32)))
      (import "" "waitable-set.wait" (func $waitable-set.wait (param i32 i32) (result i32)))

      (func (export "read") (param $r i32)
        (local $t32 i32)
        (local $ws i32)

        ;; Start a zero-length read on this stream, it should be blocked.
        (local.set $t32 (call $stream.read (local.get $r) (i32.const 100) (i32.const 0)))
        (if (i32.ne (local.get $t32) (i32.const -1 (; BLOCKED ;)))
          (then unreachable))

        ;; Wait for the zero-length read to complete, and assert the results of
        ;; completion.
        (local.set $ws (call $waitable-set.new))
        (call $waitable.join (local.get $r) (local.get $ws))
        (local.set $t32 (call $waitable-set.wait (local.get $ws) (i32.const 0)))
        (if (i32.ne (local.get $t32) (i32.const 2 (; EVENT_STREAM_READ ;)))
          (then unreachable))
        (if (i32.ne (i32.load (i32.const 0)) (local.get $r))
          (then unreachable))
        (if (i32.ne (i32.load (i32.const 4)) (i32.const 0 (; (0<<4) | COMPLETED ;) ))
          (then unreachable))
        (call $waitable.join (local.get $r) (i32.const 0))

        ;; Perform a nonzero-length-read (of 2) and assert that one item is here
        ;; immediately because that's what the writer gave us below.
        (local.set $t32 (call $stream.read (local.get $r) (i32.const 100) (i32.const 2)))
        (if (i32.ne (local.get $t32) (i32.const 0x10 (; (1<<4) | COMPLETED ;)))
          (then unreachable))

        ;; clean up
        (call $stream.drop-readable (local.get $r))
        (call $waitable-set.drop (local.get $ws))
      )
    )

    (core func $stream.read (canon stream.read $ST async))
    (core func $stream.drop-readable (canon stream.drop-readable $ST))
    (core func $waitable-set.new (canon waitable-set.new))
    (core func $waitable.join (canon waitable.join))
    (core func $waitable-set.drop (canon waitable-set.drop))

    (core instance $libc (instantiate $libc))
    (core func $waitable-set.wait (canon waitable-set.wait (memory $libc "m")))

    (core instance $i (instantiate $m
      (with "libc" (instance $libc))
      (with "" (instance
        (export "stream.read" (func $stream.read))
        (export "stream.drop-readable" (func $stream.drop-readable))
        (export "waitable-set.new" (func $waitable-set.new))
        (export "waitable.join" (func $waitable.join))
        (export "waitable-set.wait" (func $waitable-set.wait))
        (export "waitable-set.drop" (func $waitable-set.drop))
      ))
    ))

    (func (export "read") async (param "x" $ST) (canon lift (core func $i "read")))

  )
  (instance $a (instantiate $A))

  (component $B
    (import "a" (instance $a
      (export "read" (func async (param "x" $ST)))
    ))

    (core module $m
      (import "libc" "m" (memory 1))
      (import "" "stream.new" (func $stream.new (result i64)))
      (import "" "stream.write" (func $stream.write (param i32 i32 i32) (result i32)))
      (import "" "read" (func $read (param i32) (result i32)))
      (import "" "waitable-set.new" (func $waitable-set.new (result i32)))
      (import "" "waitable.join" (func $waitable.join (param i32 i32)))
      (import "" "waitable-set.drop" (func $waitable-set.drop (param i32)))
      (import "" "waitable-set.wait" (func $waitable-set.wait (param i32 i32) (result i32)))
      (import "" "subtask.drop" (func $subtask.drop (param i32)))
      (import "" "stream.drop-writable" (func $stream.drop-writable (param i32)))

      (func (export "run")
        (local $t64 i64)
        (local $t32 i32)
        (local $r i32)
        (local $w i32)
        (local $ws i32)
        (local $subtask i32)

        ;; Create a stream and split its halves.
        (local.set $t64 (call $stream.new))
        (local.set $r (i32.wrap_i64 (local.get $t64)))
        (local.set $w (i32.wrap_i64 (i64.shr_u (local.get $t64) (i64.const 32))))

        ;; Start the subtask in the above component, and it shouldn't be done
        ;; yet.
        (local.set $t32 (call $read (local.get $r)))
        (if (i32.ne (i32.and (local.get $t32) (i32.const 0xf)) (i32.const 1 (; STARTED ;)))
          (then unreachable))
        (local.set $subtask (i32.shr_u (local.get $t32) (i32.const 4)))

        ;; write of 0 values should be immediately ready
        (local.set $t32 (call $stream.write (local.get $w) (i32.const 100) (i32.const 0)))
        (if (i32.ne (local.get $t32) (i32.const 0 (; (0<<4) | COMPLETED ;)))
          (then unreachable))

        ;; write again with 0 values and it should be immediately ready
        (local.set $t32 (call $stream.write (local.get $w) (i32.const 100) (i32.const 0)))
        (if (i32.ne (local.get $t32) (i32.const 0 (; (0<<4) | COMPLETED ;)))
          (then unreachable))

        ;; write with a nonzero number of values should be blocked since this'll
        ;; wake up the reader later on.
        (local.set $t32 (call $stream.write (local.get $w) (i32.const 100) (i32.const 1)))
        (if (i32.ne (local.get $t32) (i32.const -1 (; BLOCKED ;)))
          (then unreachable))

        (local.set $ws (call $waitable-set.new))

        ;; Wait for the subtask to finish now that we've issue the write. Assert
        ;; the results of the wait as well.
        (call $waitable.join (local.get $subtask) (local.get $ws))
        (local.set $t32 (call $waitable-set.wait (local.get $ws) (i32.const 0)))
        (if (i32.ne (local.get $t32) (i32.const 1 (; EVENT_SUBTASK ;)))
          (then unreachable))
        (if (i32.ne (i32.load (i32.const 0)) (local.get $subtask))
          (then unreachable))
        (if (i32.ne (i32.load (i32.const 4)) (i32.const 0x2 (; RETURNED ;) ))
          (then unreachable))
        (call $waitable.join (local.get $subtask) (i32.const 0))

        ;; Also wait on the pending write to complete.
        (call $waitable.join (local.get $w) (local.get $ws))
        (local.set $t32 (call $waitable-set.wait (local.get $ws) (i32.const 0)))
        (if (i32.ne (local.get $t32) (i32.const 3 (; EVENT_STREAM_WRITE ;)))
          (then unreachable))
        (if (i32.ne (i32.load (i32.const 0)) (local.get $w))
          (then unreachable))
        (if (i32.ne (i32.load (i32.const 4)) (i32.const 0x11 (; (1<<4) | DROPPED ;) ))
          (then unreachable))
        (call $waitable.join (local.get $w) (i32.const 0))

        ;; clean up
        (call $subtask.drop (local.get $subtask))
        (call $stream.drop-writable (local.get $w))
        (call $waitable-set.drop (local.get $ws))
      )
    )

    (core func $stream.new (canon stream.new $ST))
    (core func $stream.write (canon stream.write $ST async))
    (core func $stream.drop-writable (canon stream.drop-writable $ST))
    (core func $read (canon lower (func $a "read") async))
    (core func $waitable-set.new (canon waitable-set.new))
    (core func $waitable.join (canon waitable.join))
    (core func $waitable-set.drop (canon waitable-set.drop))
    (core func $subtask.drop (canon subtask.drop))

    (core instance $libc (instantiate $libc))
    (core func $waitable-set.wait (canon waitable-set.wait (memory $libc "m")))

    (core instance $i (instantiate $m
      (with "libc" (instance $libc))
      (with "" (instance
        (export "stream.new" (func $stream.new))
        (export "stream.write" (func $stream.write))
        (export "read" (func $read))
        (export "waitable-set.new" (func $waitable-set.new))
        (export "waitable.join" (func $waitable.join))
        (export "waitable-set.wait" (func $waitable-set.wait))
        (export "waitable-set.drop" (func $waitable-set.drop))
        (export "subtask.drop" (func $subtask.drop))
        (export "stream.drop-writable" (func $stream.drop-writable))
      ))
    ))

    (func (export "run") async (canon lift (core func $i "run")))
  )
  (instance $b (instantiate $B (with "a" (instance $a))))

  (export "run" (func $b "run"))
)

(assert_return (invoke "run"))
