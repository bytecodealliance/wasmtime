;;! stack_switching = true
;; Testing that the creation of the necessary payload buffers works as expect,
;; even when the same continuation reference is suspended multiple times

(module
  (type $unit_to_int (func (result i32)))
  (type $int_to_unit (func (param i32)))
  (type $int_to_int (func (param i32) (result i32)))
  (type $2int_to_int (func (param i32 i32) (result i32)))
  (type $3int_to_int (func (param i32 i32 i32) (result i32)))

  (type $ct0 (cont $int_to_int))
  (type $ct1 (cont $unit_to_int))
  (type $ct2 (cont $2int_to_int))

  (global $checker (mut i32) (i32.const 0))

  (func $check_stack (param $expected i32) (param $actual i32) (result i32)
    (if (result i32)
      (i32.xor (local.get $expected) (local.get $actual))
      (then (unreachable))
      (else (local.get $actual))))

  (func $check_stack2
        (param $expected1 i32)
        (param $expected2 i32)
        (param $actual1 i32)
        (param $actual2 i32)
        (result i32 i32)
    (if
      (i32.xor (local.get $expected1) (local.get $actual1))
      (then (unreachable))
      (else))
    (if
      (i32.xor (local.get $expected2) (local.get $actual2))
      (then (unreachable))
      (else))
    (local.get $actual1)
    (local.get $actual2))


  (tag $e (param i32) (result i32))
  (tag $f (param i32 i32) (result i32 i32))

  (func $g (param $x i32) (result i32)
    (i32.add (local.get $x) (i32.const 1))
    (call $check_stack (i32.const 10))
    (suspend $e)
    (call $check_stack (i32.const 15))
    (i32.add (i32.const 5))
    (call $check_stack (i32.const 20))
    (suspend $e)
    (call $check_stack (i32.const 25))
    (i32.const 30)
    (suspend $f)
    (call $check_stack2 (i32.const 35) (i32.const 40))
    (i32.add))
  (elem declare func $g)

  (func $test (export "test") (result i32)
    (local $k1 (ref $ct0))
    (local $k2 (ref $ct1))
    (local $k3 (ref $ct2))
    (local $i i32)

    (block $on_e1 (result i32 (ref $ct0))
      (i32.const 9)
      (cont.new  $ct0 (ref.func $g))
      (cont.bind $ct0 $ct1) ;; binding 9 here as value of parameter $x of $g
      (resume $ct1 (on $e $on_e1))
      (unreachable))
    (local.set $k1)
    (call $check_stack (i32.const 10))
    (i32.add (i32.const 5))
    (call $check_stack (i32.const 15))
    (cont.bind $ct0 $ct1 (local.get $k1)) ;; binding 15
    (local.set $k2)


    (block $on_e2 (result i32 (ref $ct0))
      (resume $ct1 (on $e $on_e2) (local.get $k2))
      (unreachable))
    (local.set $k1)
    (call $check_stack (i32.const 20))
    (i32.add (i32.const 5))
    (call $check_stack (i32.const 25))
    (cont.bind $ct0 $ct1 (local.get $k1)) ;; binding 25
    (local.set $k2)
    (block $on_f (result i32 i32 (ref $ct2))
      (resume $ct1 (on $f $on_f) (local.get $k2))
      (unreachable))
    (local.set $k3)
    (call $check_stack2 (i32.const 25) (i32.const 30))
    (i32.add (i32.const 10))
    (local.set $i)
    (i32.add (i32.const 10))
    (local.get $i)
    (call $check_stack2 (i32.const 35) (i32.const 40))
    (local.get $k3)
    (cont.bind $ct2 $ct1) ;; binding 35, 40
    (resume $ct1))
)

(assert_return (invoke "test") (i32.const 75))
