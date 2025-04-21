;;! stack_switching = true
;; test proper handling of TSP pointer after a continuation returns normally

(module

  (type $unit_to_unit (func))
  (type $ct (cont $unit_to_unit))

  (type $g2_res_type (func (result (ref $ct))))
  (type $g2_res_type_ct (cont $g2_res_type))

  (tag $e1)

  (global $marker (mut i32) (i32.const 0))

  (func $update_marker (param $x i32)
    (i32.add (global.get $marker) (i32.const 1))
    (i32.mul (local.get $x))
    (global.set $marker))

  (func $g1
    (call $update_marker (i32.const 2)))
  (elem declare func $g1)

  (func $g2
    (call $update_marker (i32.const 3))

    (resume $ct (cont.new $ct (ref.func $g1)))
    (call $update_marker (i32.const 5))

    ;; This suspend only works correctly if we reset the TSP
    ;; pointer after the g1 continuation returned.
    (suspend $e1))

  (elem declare func $g2)


  (func $test (export "test") (result i32)
    (block $on_e1 (result (ref $ct))
      (resume $ct (on $e1 $on_e1) (cont.new $ct (ref.func $g2)))
      (unreachable))
    (drop)
    (global.get $marker)))

(assert_return (invoke "test") (i32.const 45))
