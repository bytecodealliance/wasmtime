;;! stack_switching = true
;; Similar to cont_nested1, but with payloads

(module

  (type $int_to_int (func (param i32) (result i32)))
  (type $ct (cont $int_to_int))

  (type $unit_to_int (func (result i32)))
  (type $ct_unit_to_int (cont $unit_to_int))

  (tag $e1 (param i32) (result i32))

  (global $marker (mut i32) (i32.const 0))

  ;; (func $update_marker (param $x i32) (result i32)
  ;;   (i32.add (global.get $marker) (i32.const 1))
  ;;   (i32.mul (local.get $x))
  ;;   (global.set $marker)
  ;;   (global.get $marker))

  (func $scramble (param $x i32) (param $y i32) (result i32)
    (i32.add (local.get $y) (i32.const 1))
    (i32.mul (local.get $x))
  )

  (func $g1 (param $x i32) (result i32)
    (call $scramble (i32.const 3) (local.get $x))
    (suspend $e1)
    (call $scramble (i32.const 5))
    (i32.add (local.get $x))
    (global.set $marker)
    (global.get $marker))
  (elem declare func $g1)

  (func $g2 (result i32)
    (local $k1 (ref $ct))
    (local $v i32)

    (block $on_e1 (result i32 (ref $ct))
      (resume $ct (on $e1 $on_e1) (i32.const 7) (cont.new $ct (ref.func $g1)))
      (unreachable))
    (local.set $k1)
    (call $scramble (i32.const 11)) ;; scramble the value received via $e1 from $g1
    (local.set $v)

    (block $on_e1_2 (result i32 (ref $ct))
      (resume $ct (on $e1 $on_e1_2) (local.get $v) (cont.new $ct (ref.func $g1)))
      (unreachable))
    (drop) ;; drop continuation, we don't intend to resume the second invocation of g1
    (call $scramble (i32.const 13))

    (resume $ct (local.get $k1))
    (i32.add (global.get $marker)))
  (elem declare func $g2)



  (func $test (export "test") (result i32)
    (resume $ct_unit_to_int (cont.new $ct_unit_to_int (ref.func $g2)))

    )
  )

(assert_return (invoke "test") (i32.const 145_670))
