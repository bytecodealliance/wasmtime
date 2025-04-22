;;! stack_switching = true
;; Simple forwarding, no payloads, no param or return values on
;; function, immediately resumed by handler

(module

  (type $unit_to_unit (func))
  (type $ct (cont $unit_to_unit))

  (type $g2_res_type (func (result (ref $ct))))
  (type $g2_res_type_ct (cont $g2_res_type))

  (tag $e1)
  (tag $e2)

  (global $marker (mut i32) (i32.const 0))

  (func $update_marker (param $x i32)
    (i32.add (global.get $marker) (i32.const 1))
    (i32.mul (local.get $x))
    (global.set $marker))

  (func $g1
    (call $update_marker (i32.const 2))
    (suspend $e1)
    (call $update_marker (i32.const 3)))
  (elem declare func $g1)

  ;; Calls $g1 as continuation, but only handles e2 rather than e1
  (func $g2
    (block $on_e2 (result (ref $ct))
      (call $update_marker (i32.const 5))
      (resume $ct (on $e2 $on_e2) (cont.new $ct (ref.func $g1)))
      (return))
    (unreachable))
  (elem declare func $g2)

  (func $g3
    (block $on_e1 (result (ref $ct))
      (call $update_marker (i32.const 7))
      (resume $ct (on $e1 $on_e1) (cont.new $ct (ref.func $g2)))
      (unreachable))
    (call $update_marker (i32.const 11))
    (resume $ct))

  (func $test (export "test") (result i32)
    (call $g3)
    (global.get $marker)))

(assert_return (invoke "test") (i32.const 2742))
