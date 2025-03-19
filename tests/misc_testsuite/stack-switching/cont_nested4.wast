;;! stack_switching = true
;; Minimal test for resuming continuation after its original parent is suspended

(module

  (type $unit_to_unit (func))
  (type $ct (cont $unit_to_unit))

  ;;(type $g2 (func (result (ref $ct))))
  ;;(type $g2_ct (cont $g2))

  (tag $e1)
  (tag $e2 (param (ref $ct)))

  (global $marker (mut i32) (i32.const 0))

  ;;(global $orphan (mut (ref $ct)))

  (func $g1
    (suspend $e1)
    (global.set $marker (i32.const 100))
    )
  (elem declare func $g1)

  (func $g2
    (block $on_e1 (result (ref $ct))
      (resume $ct (on $e1 $on_e1) (cont.new $ct (ref.func $g1)))
      (unreachable))
    (suspend $e2)
    ;; continuation becomes return value
    (unreachable))

  (elem declare func $g2)


  (func $test (export "test") (result i32)
    (block $on_e2 (result (ref $ct) (ref $ct))
      (resume $ct (on $e2 $on_e2) (cont.new $ct (ref.func $g2)))
      (unreachable))
    (drop) ;; drop the continuation (i.e., for resuming g2)
    (resume $ct) ;; resume continuation received as payload of $e2 (i.e., continuing execution of $g1)
    (global.get $marker))
)

(assert_return (invoke "test") (i32.const 100))
