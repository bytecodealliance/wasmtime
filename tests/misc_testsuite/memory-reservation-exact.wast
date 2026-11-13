;;! flags = "-O memory-reservation=65536"

;; The memory is exactly the size of the memory reservation, so every valid
;; address must pass Cranelift's bounds checks. This catches off-by-one errors
;; in static-heap bounds checks.
(module
  (memory 1 1)
  (func (export "run")
    (local $i i32)
    (loop
      (if (i32.eq (local.get $i) (i32.const 65536))
        (then (return)))
      (drop (i32.load8_u (local.get $i)))
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      br 0)))

(assert_return (invoke "run"))
