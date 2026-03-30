;;! component_model_async = true

(component definition $A
  (core module $libc (memory (export "mem") 1))
  (core instance $libc (instantiate $libc))

  (core module $m
    (import "" "mem" (memory 1))
    (import "" "stream.new" (func $stream.new (result i64)))
    (import "" "stream.read" (func $stream.read (param i32 i32 i32) (result i32)))
    (import "" "stream.write" (func $stream.write (param i32 i32 i32) (result i32)))
    (import "" "stream.cancel-read" (func $stream.cancel-read (param i32) (result i32)))
    (import "" "stream.cancel-write" (func $stream.cancel-write (param i32) (result i32)))
    (import "" "stream.drop-writable" (func $stream.drop-writable (param i32)))
    (import "" "stream.drop-readable" (func $stream.drop-readable (param i32)))
    (import "" "waitable-set.new" (func $waitable-set.new (result i32)))
    (import "" "waitable-set.wait" (func $waitable-set.wait (param i32 i32) (result i32)))
    (import "" "waitable-set.poll" (func $waitable-set.poll (param i32 i32) (result i32)))
    (import "" "waitable.join" (func $waitable.join (param i32) (param i32)))
    (import "" "waitable-set.drop" (func $waitable-set.drop (param i32)))
    (import "" "future.new" (func $future.new (result i64)))
    (import "" "future.read" (func $future.read (param i32 i32) (result i32)))
    (import "" "future.write" (func $future.write (param i32 i32) (result i32)))
    (import "" "future.cancel-read" (func $future.cancel-read (param i32) (result i32)))
    (import "" "future.cancel-write" (func $future.cancel-write (param i32) (result i32)))
    (import "" "future.drop-writable" (func $future.drop-writable (param i32)))
    (import "" "future.drop-readable" (func $future.drop-readable (param i32)))

    (table 5 funcref)
    (elem (i32.const 0)
      func
      $witness-dropped-via-cancel-read
      $witness-dropped-via-cancel-write
      $witness-dropped-via-waitable-set.wait
      $witness-dropped-via-waitable-set.poll
      $witness-dropped-via-future.cancel-write
    )

    (func $witness-dropped-via-cancel-read (param i32)
      (local $ret i32)
      (local.set $ret (call $stream.cancel-read (local.get 0)))
      (if (i32.ne (local.get $ret) (i32.const 0x01 (; DROPPED ;)))
        (then unreachable))
    )

    (func $witness-dropped-via-cancel-write (param i32)
      (local $ret i32)
      (local.set $ret (call $stream.cancel-write (local.get 0)))
      (if (i32.ne (local.get $ret) (i32.const 0x01 (; DROPPED ;)))
        (then unreachable))
    )

    (func $witness-dropped-via-waitable-set.wait (param i32)
      (local $ws i32)
      (local.set $ws (call $waitable-set.new))
      (call $waitable.join (local.get 0) (local.get $ws))
      (call $waitable-set.wait (local.get $ws) (i32.const 4))
      drop ;; ignore the event
      (call $waitable.join (local.get 0) (i32.const 0))
      (call $waitable-set.drop (local.get $ws))

      (if (i32.ne (i32.load (i32.const 4)) (local.get 0))
        (then unreachable))
      (if (i32.ne (i32.load (i32.const 8)) (i32.const 0x01 (; DROPPED ;)))
        (then unreachable))
    )

    (func $witness-dropped-via-waitable-set.poll (param i32)
      (local $ws i32)
      (local.set $ws (call $waitable-set.new))
      (call $waitable.join (local.get 0) (local.get $ws))
      (call $waitable-set.poll (local.get $ws) (i32.const 4))
      drop ;; ignore the event
      (call $waitable.join (local.get 0) (i32.const 0))
      (call $waitable-set.drop (local.get $ws))

      (if (i32.ne (i32.load (i32.const 4)) (local.get 0))
        (then unreachable))
      (if (i32.ne (i32.load (i32.const 8)) (i32.const 0x01 (; DROPPED ;)))
        (then unreachable))
    )

    (func $witness-dropped-via-future.cancel-write (param i32)
      (local $ret i32)
      (local.set $ret (call $future.cancel-write (local.get 0)))
      (if (i32.ne (local.get $ret) (i32.const 0x01 (; DROPPED ;)))
        (then unreachable))
    )

    (func (export "test-cancel-read") (param i32)
      (local $ret64 i64)
      (local $reader i32)
      (local $writer i32)
      (local $ret i32)

      ;; Create a new stream
      (local.set $ret64 (call $stream.new))
      (local.set $reader (i32.wrap_i64 (local.get $ret64)))
      (local.set $writer (i32.wrap_i64 (i64.shr_u (local.get $ret64) (i64.const 32))))

      ;; Start a read which will block (no writer ready)
      (local.set $ret (call $stream.read (local.get $reader) (i32.const 16) (i32.const 100)))
      (if (i32.ne (local.get $ret) (i32.const -1 (; BLOCKED ;)))
        (then unreachable))

      ;; Drop the writer, which queues a DROPPED event for the pending read
      (call $stream.drop-writable (local.get $writer))

      ;; Receive the dropped event
      local.get $reader
      local.get 0
      call_indirect (param i32)

      ;; attempting to read again should fail
      (drop (call $stream.read (local.get $reader) (i32.const 16) (i32.const 100)))

      unreachable
    )

    (func (export "test-cancel-write") (param i32)
      (local $ret64 i64)
      (local $reader i32)
      (local $writer i32)
      (local $ret i32)

      ;; Create a new stream
      (local.set $ret64 (call $stream.new))
      (local.set $reader (i32.wrap_i64 (local.get $ret64)))
      (local.set $writer (i32.wrap_i64 (i64.shr_u (local.get $ret64) (i64.const 32))))

      ;; Write some data (will block since no reader is ready)
      (local.set $ret (call $stream.write (local.get $writer) (i32.const 0) (i32.const 1)))
      (if (i32.ne (local.get $ret) (i32.const -1 (; BLOCKED ;)))
        (then unreachable))

      ;; Drop the reader, which queues a DROPPED event for the pending write
      (call $stream.drop-readable (local.get $reader))

      ;; Receive the dropped event
      local.get $writer
      local.get 0
      call_indirect (param i32)

      ;; attempting to write again should fail
      (drop (call $stream.write (local.get $writer) (i32.const 0) (i32.const 1)))

      unreachable
    )

    (func (export "test-cancel-future-write") (param i32)
      (local $ret64 i64)
      (local $reader i32)
      (local $writer i32)
      (local $ret i32)

      ;; Create a new future
      (local.set $ret64 (call $future.new))
      (local.set $reader (i32.wrap_i64 (local.get $ret64)))
      (local.set $writer (i32.wrap_i64 (i64.shr_u (local.get $ret64) (i64.const 32))))

      ;; Write some data (will block since no reader is ready)
      (local.set $ret (call $future.write (local.get $writer) (i32.const 0)))
      (if (i32.ne (local.get $ret) (i32.const -1 (; BLOCKED ;)))
        (then unreachable))

      ;; Drop the reader, which queues a DROPPED event for the pending write
      (call $future.drop-readable (local.get $reader))

      ;; Receive the dropped event
      local.get $writer
      local.get 0
      call_indirect (param i32)

      ;; attempting to write again should fail
      (drop (call $future.write (local.get $writer) (i32.const 0)))

      unreachable
    )
  )

  (core func $waitable-set.new (canon waitable-set.new))
  (core func $waitable.join (canon waitable.join))
  (core func $waitable-set.wait (canon waitable-set.wait (memory $libc "mem")))
  (core func $waitable-set.poll (canon waitable-set.poll (memory $libc "mem")))
  (core func $waitable-set.drop (canon waitable-set.drop))
  (type $s (stream u8))
  (core func $stream.new (canon stream.new $s))
  (core func $stream.read (canon stream.read $s async (memory $libc "mem")))
  (core func $stream.write (canon stream.write $s async (memory $libc "mem")))
  (core func $stream.cancel-read (canon stream.cancel-read $s))
  (core func $stream.cancel-write (canon stream.cancel-write $s))
  (core func $stream.drop-readable (canon stream.drop-readable $s))
  (core func $stream.drop-writable (canon stream.drop-writable $s))
  (type $f (future u8))
  (core func $future.new (canon future.new $f))
  (core func $future.read (canon future.read $f async (memory $libc "mem")))
  (core func $future.write (canon future.write $f async (memory $libc "mem")))
  (core func $future.cancel-read (canon future.cancel-read $f))
  (core func $future.cancel-write (canon future.cancel-write $f))
  (core func $future.drop-readable (canon future.drop-readable $f))
  (core func $future.drop-writable (canon future.drop-writable $f))

  (core instance $i (instantiate $m (with "" (instance
    (export "mem" (memory $libc "mem"))
    (export "stream.new" (func $stream.new))
    (export "stream.read" (func $stream.read))
    (export "stream.write" (func $stream.write))
    (export "stream.cancel-read" (func $stream.cancel-read))
    (export "stream.cancel-write" (func $stream.cancel-write))
    (export "stream.drop-writable" (func $stream.drop-writable))
    (export "stream.drop-readable" (func $stream.drop-readable))
    (export "waitable-set.new" (func $waitable-set.new))
    (export "waitable.join" (func $waitable.join))
    (export "waitable-set.wait" (func $waitable-set.wait))
    (export "waitable-set.poll" (func $waitable-set.poll))
    (export "waitable-set.drop" (func $waitable-set.drop))
    (export "future.new" (func $future.new))
    (export "future.read" (func $future.read))
    (export "future.write" (func $future.write))
    (export "future.cancel-read" (func $future.cancel-read))
    (export "future.cancel-write" (func $future.cancel-write))
    (export "future.drop-writable" (func $future.drop-writable))
    (export "future.drop-readable" (func $future.drop-readable))
  ))))

  (func (export "test-cancel-read") async (param "x" u32) (canon lift (core func $i "test-cancel-read")))
  (func (export "test-cancel-write") async (param "x" u32) (canon lift (core func $i "test-cancel-write")))
  (func (export "test-cancel-future-write") async (param "x" u32) (canon lift (core func $i "test-cancel-future-write")))
)

(component instance $A $A)
(assert_trap (invoke "test-cancel-read" (u32.const 0)) "cannot read from stream after being notified that the writable end dropped")
(component instance $A $A)
(assert_trap (invoke "test-cancel-read" (u32.const 2)) "cannot read from stream after being notified that the writable end dropped")
(component instance $A $A)
(assert_trap (invoke "test-cancel-read" (u32.const 3)) "cannot read from stream after being notified that the writable end dropped")

(component instance $A $A)
(assert_trap (invoke "test-cancel-write" (u32.const 1)) "cannot write to stream after being notified that the readable end dropped")
(component instance $A $A)
(assert_trap (invoke "test-cancel-write" (u32.const 2)) "cannot write to stream after being notified that the readable end dropped")
(component instance $A $A)
(assert_trap (invoke "test-cancel-write" (u32.const 3)) "cannot write to stream after being notified that the readable end dropped")


(component instance $A $A)
(assert_trap (invoke "test-cancel-future-write" (u32.const 2)) "cannot write to stream after being notified that the readable end dropped")
(component instance $A $A)
(assert_trap (invoke "test-cancel-future-write" (u32.const 3)) "cannot write to stream after being notified that the readable end dropped")
(component instance $A $A)
(assert_trap (invoke "test-cancel-future-write" (u32.const 4)) "cannot write to stream after being notified that the readable end dropped")
