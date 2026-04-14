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
