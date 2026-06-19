;;! component_model_async = true

;; The writable end of a future must be written to before it is dropped
;; regardless of whether the readable end is dropped first.
(component
  (core module $libc (memory (export "m") 1))
  (core instance $libc (instantiate $libc))

  (type $f (future u8))
  (core func $future.new (canon future.new $f))
  (core func $future.drop-readable (canon future.drop-readable $f))
  (core func $future.drop-writable (canon future.drop-writable $f))

  (core module $m
    (import "" "m" (memory 1))
    (import "" "future.new" (func $future.new (result i64)))
    (import "" "future.drop-readable" (func $future.drop-readable (param i32)))
    (import "" "future.drop-writable" (func $future.drop-writable (param i32)))

    (func (export "run")
      (local $tmp i64) (local $r i32) (local $w i32)
      (local.set $tmp (call $future.new))
      (local.set $r (i32.wrap_i64 (local.get $tmp)))
      (local.set $w (i32.wrap_i64 (i64.shr_u (local.get $tmp) (i64.const 32))))
      (call $future.drop-readable (local.get $r))
      (call $future.drop-writable (local.get $w))
    )
  )
  (core instance $i (instantiate $m
    (with "" (instance
      (export "m" (memory $libc "m"))
      (export "future.new" (func $future.new))
      (export "future.drop-readable" (func $future.drop-readable))
      (export "future.drop-writable" (func $future.drop-writable))
    ))
  ))
  (func (export "run") async (canon lift (core func $i "run")))
)
(assert_trap (invoke "run") "cannot drop future write end without first writing a value")
