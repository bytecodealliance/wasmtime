;;! component_model_async = true
;;! reference_types = true

(component
  (core module $M
    (import "" "stream.new" (func $stream.new (result i64)))
    (import "" "stream.read" (func $stream.read (param i32 i32 i32) (result i32)))
    (import "" "stream.write" (func $stream.write (param i32 i32 i32) (result i32)))

    (func (export "run") (param $len i32) (result i32)
      (local $s i64) (local $rx i32) (local $tx i32) (local $ret i32)

      (local.set $s (call $stream.new))
      (local.set $rx (i32.wrap_i64 (local.get $s)))
      (local.set $tx (i32.wrap_i64 (i64.shr_u (local.get $s) (i64.const 32))))

      (local.set $ret (call $stream.read
        (local.get $rx) (i32.const 0) (local.get $len)))
      (if (i32.ne (i32.const -1 (; BLOCKED ;)) (local.get $ret))
        (then unreachable))

      (local.set $ret (call $stream.write
        (local.get $tx) (i32.const 0) (local.get $len)))
      (if (i32.ne (i32.shl (local.get $len) (i32.const 4)) (local.get $ret))
        (then unreachable))

      (i32.const 42)
    )
  )
  (type $ST (stream))
  (canon stream.new $ST (core func $stream.new))
  (canon stream.read $ST async (core func $stream.read))
  (canon stream.write $ST async (core func $stream.write))
  (core instance $m (instantiate $M (with "" (instance
    (export "stream.new" (func $stream.new))
    (export "stream.read" (func $stream.read))
    (export "stream.write" (func $stream.write))
  ))))
  (func (export "run") (param "len" u32) (result u32)
    (canon lift (core func $m "run")))
)
(assert_return (invoke "run" (u32.const 0x0fffffff)) (u32.const 42))
(assert_trap (invoke "run" (u32.const 0x10000000)) "stream read/write count too large")

;; Perform a write that overflows the 32-bit index space and ensure that a trap
;; of some kind is generated.
(component
  (type $elem (tuple u64 u64 u8)) ;; 24-byte structure
  (type $s (stream $elem))

  (core module $libc (memory (export "memory") 1))
  (core instance $libc (instantiate $libc))

  (core module $m
    (import "libc" "memory" (memory 1))
    (import "" "stream.new" (func $stream.new (result i64)))
    (import "" "stream.write" (func $stream.write (param i32 i32 i32) (result i32)))

    (func (export "run") (result i32)
      (local $s i64)
      (local $w i32)
      (local.set $s (call $stream.new))
      (local.set $w (i32.wrap_i64 (i64.shr_u (local.get $s) (i64.const 32))))
      (call $stream.write
        (local.get $w)
        (i32.const 0)
        (i32.const 200000000)) ;; count: < 2^28, but 24*count > 2^32
      unreachable)

    (func (export "cb") (param i32 i32 i32) (result i32) unreachable)
  )

  (core func $stream.new (canon stream.new $s))
  (core func $stream.write (canon stream.write $s async (memory (core memory $libc "memory"))))

  (core instance $m (instantiate $m
    (with "libc" (instance $libc))
    (with "" (instance
      (export "stream.new" (func $stream.new))
      (export "stream.write" (func $stream.write))
    ))
  ))

  (func (export "run") async
    (canon lift (core func $m "run") async (callback (core func $m "cb"))))
)
(assert_trap (invoke "run") "out of bounds")
