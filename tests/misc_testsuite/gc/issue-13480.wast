;;! gc = true

(module
  (type $s (struct (field i32)))

  (import "wasmtime" "gc" (func $gc))

  (func $read_field (param (ref null $s)) (result i32)
    (struct.get $s 0 (local.get 0))
  )

  (func $keep_alive (param (ref null $s)) nop)

  (func (export "test") (result i32)
    (local $ref (ref null $s))
    (local $tmp (ref null $s))
    (local $i i32)
    (local $sum i32)

    ;; This object is loop-invariant: created before the loop, never modified
    ;; inside it.
    (local.set $ref (struct.new $s (i32.const 42)))

    (local.set $i (i32.const 0))
    (loop $loop
      ;; Add the object's field value to the sum.
      (local.set $sum
        (i32.add (local.get $sum)
                 (call $read_field (local.get $ref))))

      ;; Create another object, with a live range that overlaps the first (and
      ;; therefore should not reuse the same stack map slot).
      (local.set $tmp (struct.new $s (local.get $i)))

      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (br_if $loop (i32.lt_u (local.get $i) (i32.const 5)))
    )

    ;; After the loop: call the GC function and keep `$tmp` alive across its
    ;; safepoint.
    (call $gc)
    (call $keep_alive (local.get $tmp))

    ;; 42 * 5 loop iterations = 210
    (local.get $sum)
  )
)

(assert_return (invoke "test") (i32.const 210))
