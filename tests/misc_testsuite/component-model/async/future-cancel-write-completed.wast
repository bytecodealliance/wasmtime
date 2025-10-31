;;! component_model_async = true

;; Create a future, start a write, let it complete, and cancel the write prior
;; to receiving the completion event.
(component
  (type $f (future))

  (component $c
    (type $f (future))

    (core module $libc (memory (export "mem") 1))
    (core instance $libc (instantiate $libc))
    (core func $read (canon future.read $f async (memory $libc "mem")))
    (core func $drop-read (canon future.drop-readable $f))
    (core module $inner
      (import "" "read" (func $read (param i32 i32) (result i32)))
      (import "" "drop-read" (func $drop-read (param i32)))

      (func (export "f") (param i32)
        ;; start a read, asserting it completes with one item
        local.get 0
        i32.const 0
        call $read
        i32.const 0
        i32.ne
        if unreachable end

        ;; drop the read end
        local.get 0
        call $drop-read
      )
    )

    (core instance $i (instantiate $inner
      (with "" (instance
        (export "read" (func $read))
        (export "drop-read" (func $drop-read))
      ))
    ))

    (func (export "f") (param "x" $f) (canon lift (core func $i "f")))
  )
  (instance $c (instantiate $c))

  (core func $new (canon future.new $f))
  (core module $libc (memory (export "mem") 1))
  (core instance $libc (instantiate $libc))
  (core func $write (canon future.write $f async (memory $libc "mem")))
  (core func $cancel (canon future.cancel-write $f))
  (core func $drain (canon lower (func $c "f")))
  (core module $m
    (import "" "new" (func $new (result i64)))
    (import "" "write" (func $write (param i32 i32) (result i32)))
    (import "" "cancel" (func $cancel (param i32) (result i32)))
    (import "" "drain" (func $drain (param i32)))

    (func (export "f") (result i32)
      (local $read i32)
      (local $write i32)
      (local $new i64)

      (local.set $new (call $new))
      (local.set $read (i32.wrap_i64 (local.get $new)))
      (local.set $write (i32.wrap_i64 (i64.shr_u (local.get $new) (i64.const 32))))

      ;; start a write
      local.get $write
      i32.const 0
      call $write
      i32.const -1
      i32.ne
      if unreachable end

      ;; drain the read end
      local.get $read
      call $drain

      ;; cancel the write, returning the result
      local.get $write
      call $cancel
    )
  )

  (core instance $i (instantiate $m
    (with "" (instance
      (export "new" (func $new))
      (export "write" (func $write))
      (export "cancel" (func $cancel))
      (export "drain" (func $drain))
    ))
  ))

  (func (export "f") (result u32) (canon lift (core func $i "f")))
)

(assert_return (invoke "f") (u32.const 0))
