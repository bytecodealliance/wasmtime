;;! stack_switching = true
;; Testing cont.bind on continuations received from suspending rather than cont.new.

(module
  (type $unit_to_int (func (result i32)))
  (type $int_to_unit (func (param i32)))
  (type $int_to_int (func (param i32) (result i32)))
  (type $2int_to_int (func (param i32 i32) (result i32)))
  (type $3int_to_int (func (param i32 i32 i32) (result i32)))

  (type $ct0 (cont $unit_to_int))
  (type $ct1 (cont $3int_to_int))
  (type $ct2 (cont $2int_to_int))
  (type $ct3 (cont $int_to_int))

  (tag $e (param i32 i32) (result i32 i32 i32))

  (func $g (result i32)
    (suspend $e (i32.const 5) (i32.const 15))
    (i32.add)
    (i32.add))
  (elem declare func $g)

  (func $test (export "test") (result i32)
    (local $k (ref $ct1))
    (i32.const 35) ;; to be consumed by second call to cont.resume
    (i32.const 45) ;; to be consumed by second call to cont.bind
    (block $on_e (result i32 i32 (ref $ct1))
      (resume $ct0 (on $e $on_e) (cont.new  $ct0 (ref.func $g)))
      (unreachable))
    ;; on_e:
    (local.set $k)
    (i32.add) ;; add two values received from $e, leave on stack to be consumed by first call to cont.bind
    (local.get $k)
    (cont.bind $ct1 $ct2) ;; consumes the result (= 20) of the addition two lines earlier
    (cont.bind $ct2 $ct3) ;; consumes the constant value 45 put on stack earlier
    (resume $ct3) ;; consumes the constant value 35 put on stack earlier
    )
)

(assert_return (invoke "test") (i32.const 100))
