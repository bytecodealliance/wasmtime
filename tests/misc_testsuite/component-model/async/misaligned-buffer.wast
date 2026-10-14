;;! component_model_async = true

;; Buffer pointers are validated at the time a `{stream,future}.{read,write}`
;; is issued regardless of the state of the other end of the stream or future.
(component definition $C
  (core module $libc (memory (export "mem") 1))
  (core instance $libc (instantiate $libc))
  (type $ST (stream u32))
  (type $FT (future u32))
  (core func $stream.new (canon stream.new $ST))
  (core func $stream.read
    (canon stream.read $ST async (memory (core memory $libc "mem"))))
  (core func $stream.write
    (canon stream.write $ST async (memory (core memory $libc "mem"))))
  (core func $stream.drop-readable (canon stream.drop-readable $ST))
  (core func $future.new (canon future.new $FT))
  (core func $future.read
    (canon future.read $FT async (memory (core memory $libc "mem"))))
  (core func $future.write
    (canon future.write $FT async (memory (core memory $libc "mem"))))
  (core module $M
    (import "" "stream.new" (func $stream.new (result i64)))
    (import "" "stream.read" (func $stream.read (param i32 i32 i32) (result i32)))
    (import "" "stream.write" (func $stream.write (param i32 i32 i32) (result i32)))
    (import "" "stream.drop-readable" (func $stream.drop-readable (param i32)))
    (import "" "future.new" (func $future.new (result i64)))
    (import "" "future.read" (func $future.read (param i32 i32) (result i32)))
    (import "" "future.write" (func $future.write (param i32 i32) (result i32)))

    ;; Reads `n` items to `p` from a new stream with no writer.
    (func (export "stream-read") (param $p i32) (param $n i32) (result i32)
      (call $stream.read (i32.wrap_i64 (call $stream.new))
        (local.get $p) (local.get $n)))

    ;; Writes `n` items from `p` to a new stream with no reader.
    (func (export "stream-write") (param $p i32) (param $n i32) (result i32)
      (call $stream.write
        (i32.wrap_i64 (i64.shr_u (call $stream.new) (i64.const 32)))
        (local.get $p) (local.get $n)))

    ;; Writes `n` items from `p` to a new stream whose readable end has been
    ;; dropped.
    (func (export "stream-write-dropped") (param $p i32) (param $n i32)
      (result i32)
      (local $rw i64)
      (local.set $rw (call $stream.new))
      (call $stream.drop-readable (i32.wrap_i64 (local.get $rw)))
      (call $stream.write
        (i32.wrap_i64 (i64.shr_u (local.get $rw) (i64.const 32)))
        (local.get $p) (local.get $n)))

    ;; Reads to `p` from a new future with no writer.
    (func (export "future-read") (param $p i32) (result i32)
      (call $future.read (i32.wrap_i64 (call $future.new)) (local.get $p)))

    ;; Writes from `p` to a new future with no reader.
    (func (export "future-write") (param $p i32) (result i32)
      (call $future.write
        (i32.wrap_i64 (i64.shr_u (call $future.new) (i64.const 32)))
        (local.get $p)))
  )
  (core instance $m (instantiate $M (with "" (instance
    (export "stream.new" (func $stream.new))
    (export "stream.read" (func $stream.read))
    (export "stream.write" (func $stream.write))
    (export "stream.drop-readable" (func $stream.drop-readable))
    (export "future.new" (func $future.new))
    (export "future.read" (func $future.read))
    (export "future.write" (func $future.write))))))
  (func (export "stream-read") (param "p" u32) (param "n" u32) (result u32)
    (canon lift (core func $m "stream-read")))
  (func (export "stream-write") (param "p" u32) (param "n" u32) (result u32)
    (canon lift (core func $m "stream-write")))
  (func (export "stream-write-dropped") (param "p" u32) (param "n" u32)
    (result u32)
    (canon lift (core func $m "stream-write-dropped")))
  (func (export "future-read") (param "p" u32) (result u32)
    (canon lift (core func $m "future-read")))
  (func (export "future-write") (param "p" u32) (result u32)
    (canon lift (core func $m "future-write")))
)

;; Misaligned: trap immediately, whether the operation would otherwise block or
;; report that the other end was dropped.
(component instance $i1 $C)
(assert_trap (invoke "stream-read" (u32.const 1) (u32.const 1)) "pointer not aligned")
(component instance $i2 $C)
(assert_trap (invoke "stream-write" (u32.const 1) (u32.const 1)) "pointer not aligned")
(component instance $i3 $C)
(assert_trap (invoke "stream-write-dropped" (u32.const 1) (u32.const 1)) "pointer not aligned")
(component instance $i4 $C)
(assert_trap (invoke "future-read" (u32.const 1)) "pointer not aligned")
(component instance $i5 $C)
(assert_trap (invoke "future-write" (u32.const 1)) "pointer not aligned")

;; Aligned: the operation blocks, or reports DROPPED.
(component instance $i6 $C)
(assert_return (invoke "stream-read" (u32.const 4) (u32.const 1)) (u32.const 0xffffffff))
(assert_return (invoke "stream-write" (u32.const 4) (u32.const 1)) (u32.const 0xffffffff))
(assert_return (invoke "stream-write-dropped" (u32.const 4) (u32.const 1)) (u32.const 1))
(assert_return (invoke "future-read" (u32.const 4)) (u32.const 0xffffffff))
(assert_return (invoke "future-write" (u32.const 4)) (u32.const 0xffffffff))
