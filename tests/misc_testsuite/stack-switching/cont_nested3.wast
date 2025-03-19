;;! stack_switching = true
;; Minimal test for resuming continuation after its original parent is gone

(module

  (type $unit_to_unit (func))
  (type $ct (cont $unit_to_unit))

  (type $g2 (func (result (ref $ct))))
  (type $g2_ct (cont $g2))

  (tag $e1)
  ;;(tag $e2 (param (ref $ct)))

  (global $marker (mut i32) (i32.const 0))

  ;;(global $orphan (mut (ref $ct)))

  (func $g1
    (suspend $e1)
    (global.set $marker (i32.const 100))
    )
  (elem declare func $g1)

  (func $g2 (result (ref $ct))
    (block $on_e1 (result (ref $ct))
      (resume $ct (on $e1 $on_e1) (cont.new $ct (ref.func $g1)))
      (unreachable))
    ;; continuation becomes return value
    )

  (elem declare func $g2)


  (func $test (export "test") (result i32)
    (resume $g2_ct (cont.new $g2_ct (ref.func $g2)))
    (resume $ct) ;; resume return value of $g2
    (global.get $marker))
  )

(assert_return (invoke "test") (i32.const 100))
