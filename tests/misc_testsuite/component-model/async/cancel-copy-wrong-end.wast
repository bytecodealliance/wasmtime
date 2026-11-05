;;! component_model_async = true

;; `{stream,future}.cancel-write` must trap when given a readable end, and
;; `{stream,future}.cancel-read` must trap when given a writable end, even if
;; that end has a copy in progress.

(component definition $C
  (core module $libc (memory (export "mem") 1))
  (core instance $libc (instantiate $libc))

  (type $s (stream u8))
  (type $f (future u8))

  (core func $stream.new (canon stream.new $s))
  (core func $stream.read (canon stream.read $s async (memory (core memory $libc "mem"))))
  (core func $stream.write (canon stream.write $s async (memory (core memory $libc "mem"))))
  (core func $stream.cancel-read (canon stream.cancel-read $s))
  (core func $stream.cancel-write (canon stream.cancel-write $s))
  (core func $future.new (canon future.new $f))
  (core func $future.read (canon future.read $f async (memory (core memory $libc "mem"))))
  (core func $future.write (canon future.write $f async (memory (core memory $libc "mem"))))
  (core func $future.cancel-read (canon future.cancel-read $f))
  (core func $future.cancel-write (canon future.cancel-write $f))

  (core module $m
    (import "" "stream.new" (func $stream.new (result i64)))
    (import "" "stream.read" (func $stream.read (param i32 i32 i32) (result i32)))
    (import "" "stream.write" (func $stream.write (param i32 i32 i32) (result i32)))
    (import "" "stream.cancel-read" (func $stream.cancel-read (param i32) (result i32)))
    (import "" "stream.cancel-write" (func $stream.cancel-write (param i32) (result i32)))
    (import "" "future.new" (func $future.new (result i64)))
    (import "" "future.read" (func $future.read (param i32 i32) (result i32)))
    (import "" "future.write" (func $future.write (param i32 i32) (result i32)))
    (import "" "future.cancel-read" (func $future.cancel-read (param i32) (result i32)))
    (import "" "future.cancel-write" (func $future.cancel-write (param i32) (result i32)))

    ;; Start a read on a new stream's readable end, which blocks since nothing
    ;; has been written, then pass that readable end to `stream.cancel-write`.
    (func (export "stream-cancel-write-on-readable-end")
      (local $readable i32)
      (local.set $readable (i32.wrap_i64 (call $stream.new)))
      (if (i32.ne (call $stream.read (local.get $readable) (i32.const 0) (i32.const 4))
                  (i32.const -1 (; BLOCKED ;)))
        (then unreachable))
      (drop (call $stream.cancel-write (local.get $readable)))
    )

    ;; Start a write on a new stream's writable end, which blocks since
    ;; nothing is reading, then pass that writable end to `stream.cancel-read`.
    (func (export "stream-cancel-read-on-writable-end")
      (local $writable i32)
      (local.set $writable (i32.wrap_i64 (i64.shr_u (call $stream.new) (i64.const 32))))
      (if (i32.ne (call $stream.write (local.get $writable) (i32.const 0) (i32.const 4))
                  (i32.const -1 (; BLOCKED ;)))
        (then unreachable))
      (drop (call $stream.cancel-read (local.get $writable)))
    )

    ;; Same as above, for futures.
    (func (export "future-cancel-write-on-readable-end")
      (local $readable i32)
      (local.set $readable (i32.wrap_i64 (call $future.new)))
      (if (i32.ne (call $future.read (local.get $readable) (i32.const 0))
                  (i32.const -1 (; BLOCKED ;)))
        (then unreachable))
      (drop (call $future.cancel-write (local.get $readable)))
    )

    (func (export "future-cancel-read-on-writable-end")
      (local $writable i32)
      (local.set $writable (i32.wrap_i64 (i64.shr_u (call $future.new) (i64.const 32))))
      (if (i32.ne (call $future.write (local.get $writable) (i32.const 0))
                  (i32.const -1 (; BLOCKED ;)))
        (then unreachable))
      (drop (call $future.cancel-read (local.get $writable)))
    )
  )

  (core instance $i (instantiate $m (with "" (instance
    (export "stream.new" (func $stream.new))
    (export "stream.read" (func $stream.read))
    (export "stream.write" (func $stream.write))
    (export "stream.cancel-read" (func $stream.cancel-read))
    (export "stream.cancel-write" (func $stream.cancel-write))
    (export "future.new" (func $future.new))
    (export "future.read" (func $future.read))
    (export "future.write" (func $future.write))
    (export "future.cancel-read" (func $future.cancel-read))
    (export "future.cancel-write" (func $future.cancel-write))
  ))))

  (func (export "stream-cancel-write-on-readable-end") async
    (canon lift (core func $i "stream-cancel-write-on-readable-end")))
  (func (export "stream-cancel-read-on-writable-end") async
    (canon lift (core func $i "stream-cancel-read-on-writable-end")))
  (func (export "future-cancel-write-on-readable-end") async
    (canon lift (core func $i "future-cancel-write-on-readable-end")))
  (func (export "future-cancel-read-on-writable-end") async
    (canon lift (core func $i "future-cancel-read-on-writable-end")))
)

(component instance $C $C)
(assert_trap (invoke "stream-cancel-write-on-readable-end") "passed read end")
(component instance $C $C)
(assert_trap (invoke "stream-cancel-read-on-writable-end") "passed write end")
(component instance $C $C)
(assert_trap (invoke "future-cancel-write-on-readable-end") "passed read end")
(component instance $C $C)
(assert_trap (invoke "future-cancel-read-on-writable-end") "passed write end")
