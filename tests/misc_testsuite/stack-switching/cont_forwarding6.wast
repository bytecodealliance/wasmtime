;;! stack_switching = true
;; The resumed continuation suspends again (from the same inner function), using the same tag.
;; We install a new handler at the outermost level, which should be forwarded to correctly.

(module

  (type $int_to_int (func (param i32) (result i32)))
  (type $unit_to_int (func (result i32)))
  (type $ct0 (cont $int_to_int))
  (type $ct1 (cont $unit_to_int))

  (tag $e1 (param i32) (result i32))
  (tag $e2)

  (func $g1 (param $x i32) (result i32)
    (i32.add (local.get $x) (i32.const 1))
    (suspend $e1)
    (i32.add (i32.const 1))
    (suspend $e1)
    (i32.add (i32.const 1)))
  (elem declare func $g1)

  ;; Calls $g1 as continuation, but only handles e2 rather than e1
  (func $g2 (param $x i32) (result i32)
    (block $on_e2 (result (ref $ct1))
      ;;(call $update_marker (i32.const 5))
      (i32.add (local.get $x) (i32.const 1))
      (resume $ct0 (on $e2 $on_e2) (cont.new $ct0 (ref.func $g1)))
      (i32.add (i32.const 1))
      (return))
    (unreachable))
  (elem declare func $g2)

  (func $g3 (param $x i32) (result i32)
    (local $k1 (ref $ct0))
    (block $on_e1 (result i32 (ref $ct0))
      (i32.add (local.get $x) (i32.const 1))
      (resume $ct0 (on $e1 $on_e1) (cont.new $ct0 (ref.func $g2)))
      (unreachable))
    (local.set $k1)
    (i32.add (i32.const 1))

    ;; g1 will suspend again. We install a new handler for $e1
    (block $on_e1_2 (param i32) (result i32 (ref $ct0))
      (resume $ct0 (on $e1 $on_e1_2) (local.get $k1))
      (unreachable))
    (local.set $k1)
    (i32.add (i32.const 1))

    (resume $ct0 (local.get $k1))
    (i32.add (i32.const 1)))

  (func $test (export "test") (result i32)
    (call $g3 (i32.const 1))))

(assert_return (invoke "test") (i32.const 10))
