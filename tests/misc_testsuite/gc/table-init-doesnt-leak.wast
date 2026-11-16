;;! gc = true
;;! bulk_memory = true
;;! flags = """
;;!   -O memory-may-move=n
;;!   -O memory-reservation=65536
;;!   -O memory-reservation-for-growth=0
;;! """

(module
  (table 1 arrayref)

  (type $a (array i31ref))

  (elem $e arrayref
    (array.new $a (ref.i31 (i32.const 0)) (i32.const 200)))

  (func (export "run")
    (local $i i32)
    (loop $l
      (table.init $e (i32.const 0) (i32.const 0) (i32.const 1))
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (br_if $l (i32.lt_u (local.get $i) (i32.const 200))))))

(assert_return (invoke "run"))
