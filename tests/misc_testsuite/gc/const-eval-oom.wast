;;! gc = true
;;! bulk_memory = true
;;! flags = """
;;!   -O memory-may-move=n
;;!   -O memory-reservation=65536
;;!   -O memory-reservation-for-growth=0
;;!   -O gc-heap-may-move=n
;;!   -O gc-heap-reservation=65536
;;!   -O gc-heap-reservation-for-growth=0
;;! """

;; Running out of GC heap while evaluating constant expressions during
;; instantiation fails the instantiation instead of panicking.

;; In a global initializer.
(assert_trap
  (module
    (table 100 arrayref)
    (type $a (array i31ref))
    (global (ref $a) (array.new_default $a (i32.const 10000000))))
  "GC heap out of memory")

;; In an element segment.
(assert_trap
  (module
    (type $a (array i31ref))
    (elem arrayref
      (array.new_default $a (i32.const 100))
      (array.new_default $a (i32.const 10000))
      (array.new_default $a (i32.const 10000))
      (array.new_default $a (i32.const 10000))
      (array.new_default $a (i32.const 1000000))))
  "GC heap out of memory")
