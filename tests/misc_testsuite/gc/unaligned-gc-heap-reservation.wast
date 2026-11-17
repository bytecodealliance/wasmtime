;;! gc = true
;;! flags = """
;;!    -C collector=drc
;;!    -O memory-reservation=131071
;;!    -O memory-may-move=n
;;!    -O gc-heap-reservation=131071
;;!    -O gc-heap-may-move=n
;;!    -O pooling-max-memory-size=131071
;;! """

;; A GC heap that may not move can't grow beyond its reservation, even when the
;; reservation isn't a multiple of the host page size. Filling it with arrays
;; up to its end runs out of memory, and every array allocated before that
;; point, including the ones at the very end of the heap, is accessible.

(module
  (type $a (array (mut i8)))
  (table $t 10000 (ref null $a))
  (global $g (mut (ref null $a)) (ref.null $a))

  ;; Allocates one $pad-byte array, then $n arrays of $len bytes that are
  ;; kept alive in a table, writing and reading the last element of each.
  (func (export "run") (param $n i32) (param $len i32) (param $pad i32) (result i32)
    (local $i i32) (local $r (ref null $a))
    (global.set $g (array.new_default $a (local.get $pad)))
    (loop $l
      (local.set $r (array.new_default $a (local.get $len)))
      (table.set $t (local.get $i) (local.get $r))
      (array.set $a (local.get $r)
        (i32.sub (local.get $len) (i32.const 1))
        (i32.const 7))
      (if (i32.ne (array.get_u $a (local.get $r)
                    (i32.sub (local.get $len) (i32.const 1)))
                  (i32.const 7))
        (then (unreachable)))
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (br_if $l (i32.lt_u (local.get $i) (local.get $n))))
    (local.get $i))
)

;; 4094 arrays would fill a 131072-byte heap exactly.
(assert_trap (invoke "run" (i32.const 4094) (i32.const 4) (i32.const 12)) "GC heap out of memory")
