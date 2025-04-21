;;! stack_switching = true
;; Simple test for cont.bind: cont.bind supplies 0 arguments

(module
  (type $unit_to_int (func (result i32)))
  (type $int_int_to_int (func (param i32 i32) (result i32)))
  (type $int_to_int (func (param i32) (result i32)))

  (type $ct0 (cont $unit_to_int))
  (type $ct1 (cont $int_int_to_int))

  (tag $e)

  (func $g (param $x i32) (param $y i32) (result i32)
    (suspend $e)
    (i32.add (local.get $x) (local.get $y)))
  (elem declare func $g)

  (func $test (export "test") (result i32)
    (block $on_e (result (ref $ct0))
      (i32.const 49) ;; consumed by resume
      (i32.const 51) ;; consumed by resume
      (cont.new $ct1 (ref.func $g))
      (cont.bind $ct1 $ct1)
      (resume $ct1 (on $e $on_e))
      (unreachable))
    ;; on_e
    (resume $ct0))
)

(assert_return (invoke "test") (i32.const 100))
