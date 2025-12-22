;;! component_model_async = true
;;! component_model_async_builtins = true

(component
  (core module $libc
     (func (export "realloc") (param i32 i32 i32 i32) (result i32)
        unreachable
     )
     (memory (export "m") 1)
  )
  (core instance $libc (instantiate $libc))

  (type $s (future string))
  (core func $future.new (canon future.new $s))
  (core func $future.read (canon future.read $s async (memory $libc "m") (realloc (func $libc "realloc"))))
  (core func $future.write (canon future.write $s async (memory $libc "m")))

  (core module $m
    (import "" "m" (memory 1))
    (import "" "future.new" (func $future.new (result i64)))
    (import "" "future.read" (func $future.read (param i32 i32) (result i32)))
    (import "" "future.write" (func $future.write (param i32 i32) (result i32)))

    (func (export "run")
      (local $tmp i64)
      (local $r i32)
      (local $w i32)
      (local.set $tmp (call $future.new))

      (local.set $r (i32.wrap_i64 (local.get $tmp)))
      (local.set $w (i32.wrap_i64 (i64.shr_u (local.get $tmp) (i64.const 32))))

      (call $future.read (local.get $r) (i32.const 0))
      i32.const -1 ;; BLOCKED
      i32.ne
      if unreachable end

      (call $future.write (local.get $w) (i32.const 0))
      drop
    )
  )

  (core instance $i (instantiate $m
    (with "" (instance
      (export "m" (memory $libc "m"))
      (export "future.new" (func $future.new))
      (export "future.read" (func $future.read))
      (export "future.write" (func $future.write))
    ))
  ))

  (func (export "run") (canon lift (core func $i "run")))
)

(assert_trap (invoke "run") "cannot read from and write to intra-component future with non-numeric payload")
