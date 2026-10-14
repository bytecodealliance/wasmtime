;;! component_model_async = true
;;! component_model_more_async_builtins = true

;; A synchronous `{stream,future}.{read,write}` on an end which is in a waitable
;; set traps on entry, even though here the copy could complete without blocking
;; since the other end already has a pending async copy.
(component definition $C
  (core module $libc (memory (export "memory") 1))
  (core instance $libc (instantiate $libc))
  (type $s (stream u8))
  (type $f (future u8))
  (core func $waitable-set.new (canon waitable-set.new))
  (core func $waitable.join (canon waitable.join))
  (core func $stream.new (canon stream.new $s))
  (core func $stream.read (canon stream.read $s (memory (core memory $libc "memory"))))
  (core func $stream.write (canon stream.write $s (memory (core memory $libc "memory"))))
  (core func $stream.read-async (canon stream.read $s async (memory (core memory $libc "memory"))))
  (core func $stream.write-async (canon stream.write $s async (memory (core memory $libc "memory"))))
  (core func $future.new (canon future.new $f))
  (core func $future.read (canon future.read $f (memory (core memory $libc "memory"))))
  (core func $future.write (canon future.write $f (memory (core memory $libc "memory"))))
  (core func $future.read-async (canon future.read $f async (memory (core memory $libc "memory"))))
  (core func $future.write-async (canon future.write $f async (memory (core memory $libc "memory"))))
  (core module $m
    (import "" "waitable-set.new" (func $waitable-set.new (result i32)))
    (import "" "waitable.join" (func $waitable.join (param i32 i32)))
    (import "" "stream.new" (func $stream.new (result i64)))
    (import "" "stream.read" (func $stream.read (param i32 i32 i32) (result i32)))
    (import "" "stream.write" (func $stream.write (param i32 i32 i32) (result i32)))
    (import "" "stream.read-async" (func $stream.read-async (param i32 i32 i32) (result i32)))
    (import "" "stream.write-async" (func $stream.write-async (param i32 i32 i32) (result i32)))
    (import "" "future.new" (func $future.new (result i64)))
    (import "" "future.read" (func $future.read (param i32 i32) (result i32)))
    (import "" "future.write" (func $future.write (param i32 i32) (result i32)))
    (import "" "future.read-async" (func $future.read-async (param i32 i32) (result i32)))
    (import "" "future.write-async" (func $future.write-async (param i32 i32) (result i32)))

    (func $readable (param i64) (result i32)
      (i32.wrap_i64 (local.get 0)))
    (func $writable (param i64) (result i32)
      (i32.wrap_i64 (i64.shr_u (local.get 0) (i64.const 32))))
    (func $expect-blocked (param i32)
      (if (i32.ne (local.get 0) (i32.const -1)) (then unreachable)))
    ;; Adds `$end` to a new waitable set and returns it.
    (func $joined (param $end i32) (result i32)
      (call $waitable.join (local.get $end) (call $waitable-set.new))
      (local.get $end))

    (func (export "stream.read")
      (local $pair i64)
      (local.set $pair (call $stream.new))
      (call $expect-blocked
        (call $stream.write-async (call $writable (local.get $pair)) (i32.const 0) (i32.const 4)))
      (drop (call $stream.read
        (call $joined (call $readable (local.get $pair))) (i32.const 0) (i32.const 4)))
      unreachable)
    (func (export "stream.write")
      (local $pair i64)
      (local.set $pair (call $stream.new))
      (call $expect-blocked
        (call $stream.read-async (call $readable (local.get $pair)) (i32.const 0) (i32.const 4)))
      (drop (call $stream.write
        (call $joined (call $writable (local.get $pair))) (i32.const 0) (i32.const 4)))
      unreachable)
    (func (export "future.read")
      (local $pair i64)
      (local.set $pair (call $future.new))
      (call $expect-blocked
        (call $future.write-async (call $writable (local.get $pair)) (i32.const 0)))
      (drop (call $future.read
        (call $joined (call $readable (local.get $pair))) (i32.const 0)))
      unreachable)
    (func (export "future.write")
      (local $pair i64)
      (local.set $pair (call $future.new))
      (call $expect-blocked
        (call $future.read-async (call $readable (local.get $pair)) (i32.const 0)))
      (drop (call $future.write
        (call $joined (call $writable (local.get $pair))) (i32.const 0)))
      unreachable)
  )
  (core instance $m (instantiate $m
    (with "" (instance
      (export "waitable-set.new" (func $waitable-set.new))
      (export "waitable.join" (func $waitable.join))
      (export "stream.new" (func $stream.new))
      (export "stream.read" (func $stream.read))
      (export "stream.write" (func $stream.write))
      (export "stream.read-async" (func $stream.read-async))
      (export "stream.write-async" (func $stream.write-async))
      (export "future.new" (func $future.new))
      (export "future.read" (func $future.read))
      (export "future.write" (func $future.write))
      (export "future.read-async" (func $future.read-async))
      (export "future.write-async" (func $future.write-async))
    ))
  ))
  (func (export "stream-read") async (canon lift (core func $m "stream.read")))
  (func (export "stream-write") async (canon lift (core func $m "stream.write")))
  (func (export "future-read") async (canon lift (core func $m "future.read")))
  (func (export "future-write") async (canon lift (core func $m "future.write")))
)

(component instance $i $C)
(assert_trap (invoke "stream-read") "waitable cannot be used synchronously while added to a waitable set")
(component instance $i $C)
(assert_trap (invoke "stream-write") "waitable cannot be used synchronously while added to a waitable set")
(component instance $i $C)
(assert_trap (invoke "future-read") "waitable cannot be used synchronously while added to a waitable set")
(component instance $i $C)
(assert_trap (invoke "future-write") "waitable cannot be used synchronously while added to a waitable set")
