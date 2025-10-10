;;! component_model_async = true

;; Create a future, start a write, drop the read end, and cancel the write.
(component
  (type $f (future))
  (core func $new (canon future.new $f))
  (core module $libc (memory (export "mem") 1))
  (core instance $libc (instantiate $libc))
  (core func $write (canon future.write $f async (memory $libc "mem")))
  (core func $cancel (canon future.cancel-write $f))
  (core func $drop-read (canon future.drop-readable $f))
  (core module $m
    (import "" "new" (func $new (result i64)))
    (import "" "write" (func $write (param i32 i32) (result i32)))
    (import "" "cancel" (func $cancel (param i32) (result i32)))
    (import "" "drop-read" (func $drop-read (param i32)))

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

      ;; drop the read end
      local.get $read
      call $drop-read

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
      (export "drop-read" (func $drop-read))
    ))
  ))

  (func (export "f") (result u32) (canon lift (core func $i "f")))
)

(assert_return (invoke "f") (u32.const 1)) ;; expect DROPPED status (not CANCELLED)
