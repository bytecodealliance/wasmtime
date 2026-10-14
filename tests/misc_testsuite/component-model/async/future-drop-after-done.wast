;;! component_model_async = true

;; Once a future's value has been transferred, and the end that completed last
;; has received its COMPLETED event, both ends are done. Dropping one end must
;; then not deliver a DROPPED event to the other.
(component
  (core module $libc (memory (export "memory") 1))
  (core instance $libc (instantiate $libc))
  (type $FT (future u8))
  (core func $future.new (canon future.new $FT))
  (core func $future.read
    (canon future.read $FT async (memory (core memory $libc "memory"))))
  (core func $future.write
    (canon future.write $FT async (memory (core memory $libc "memory"))))
  (core func $future.drop-readable (canon future.drop-readable $FT))
  (core func $future.drop-writable (canon future.drop-writable $FT))
  (core func $waitable-set.new (canon waitable-set.new))
  (core func $waitable-set.poll
    (canon waitable-set.poll (memory (core memory $libc "memory"))))
  (core func $waitable.join (canon waitable.join))
  (core module $m
    (import "" "future.new" (func $future.new (result i64)))
    (import "" "future.read" (func $future.read (param i32 i32) (result i32)))
    (import "" "future.write" (func $future.write (param i32 i32) (result i32)))
    (import "" "future.drop-readable" (func $future.drop-readable (param i32)))
    (import "" "future.drop-writable" (func $future.drop-writable (param i32)))
    (import "" "waitable-set.new" (func $waitable-set.new (result i32)))
    (import "" "waitable-set.poll" (func $waitable-set.poll (param i32 i32) (result i32)))
    (import "" "waitable.join" (func $waitable.join (param i32 i32)))

    ;; The write blocks and the read completes it. After the writer receives
    ;; FUTURE_WRITE, dropping it must not notify the reader.
    (func (export "drop-writable") (result i32)
      (local $futures i64) (local $r i32) (local $w i32) (local $set i32)
      (local.set $futures (call $future.new))
      (local.set $r (i32.wrap_i64 (local.get $futures)))
      (local.set $w (i32.wrap_i64 (i64.shr_u (local.get $futures) (i64.const 32))))
      (if (i32.ne (call $future.write (local.get $w) (i32.const 0))
            (i32.const -1 (; BLOCKED ;)))
        (then unreachable))
      (if (i32.ne (call $future.read (local.get $r) (i32.const 0))
            (i32.const 0 (; COMPLETED ;)))
        (then unreachable))
      (local.set $set (call $waitable-set.new))
      (call $waitable.join (local.get $w) (local.get $set))
      (if (i32.ne (call $waitable-set.poll (local.get $set) (i32.const 0))
            (i32.const 5 (; FUTURE_WRITE ;)))
        (then unreachable))
      (call $waitable.join (local.get $r) (local.get $set))
      (call $future.drop-writable (local.get $w))
      ;; Expect no event (0).
      (call $waitable-set.poll (local.get $set) (i32.const 0)))

    ;; The read blocks and the write completes it. After the reader receives
    ;; FUTURE_READ, dropping it must not notify the writer.
    (func (export "drop-readable") (result i32)
      (local $futures i64) (local $r i32) (local $w i32) (local $set i32)
      (local.set $futures (call $future.new))
      (local.set $r (i32.wrap_i64 (local.get $futures)))
      (local.set $w (i32.wrap_i64 (i64.shr_u (local.get $futures) (i64.const 32))))
      (if (i32.ne (call $future.read (local.get $r) (i32.const 0))
            (i32.const -1 (; BLOCKED ;)))
        (then unreachable))
      (if (i32.ne (call $future.write (local.get $w) (i32.const 0))
            (i32.const 0 (; COMPLETED ;)))
        (then unreachable))
      (local.set $set (call $waitable-set.new))
      (call $waitable.join (local.get $r) (local.get $set))
      (if (i32.ne (call $waitable-set.poll (local.get $set) (i32.const 0))
            (i32.const 4 (; FUTURE_READ ;)))
        (then unreachable))
      (call $waitable.join (local.get $w) (local.get $set))
      (call $future.drop-readable (local.get $r))
      ;; Expect no event (0).
      (call $waitable-set.poll (local.get $set) (i32.const 0)))
  )
  (core instance $i (instantiate $m (with "" (instance
    (export "future.new" (func $future.new))
    (export "future.read" (func $future.read))
    (export "future.write" (func $future.write))
    (export "future.drop-readable" (func $future.drop-readable))
    (export "future.drop-writable" (func $future.drop-writable))
    (export "waitable-set.new" (func $waitable-set.new))
    (export "waitable-set.poll" (func $waitable-set.poll))
    (export "waitable.join" (func $waitable.join))))))
  (func (export "drop-writable") (result u32)
    (canon lift (core func $i "drop-writable")))
  (func (export "drop-readable") (result u32)
    (canon lift (core func $i "drop-readable")))
)

(assert_return (invoke "drop-writable") (u32.const 0))
(assert_return (invoke "drop-readable") (u32.const 0))
