;;! stack_switching = true
;; This file tests passing arguments to functions used has continuations and
;; returning values from such continuations on ordinary (i.e., non-suspend) exit

(module

  (type $unit_to_unit (func))
  (type $unit_to_int (func (result i32)))
  (type $int_to_unit (func (param i32)))
  (type $int_to_int (func (param i32) (result i32)))


  (type $f1_t (func (param i32) (result i32)))
  (type $f1_ct (cont $f1_t))

  (type $f2_t (func (param i32) (result i32)))
  (type $f2_ct (cont $f2_t))

  (type $f3_t (func (param i32) (result i32)))
  (type $f3_ct (cont $f3_t))

  (type $res_unit_to_unit (cont $unit_to_unit))
  (type $res_int_to_unit (cont $int_to_unit))
  (type $res_int_to_int (cont $int_to_int))
  (type $res_unit_to_int (cont $unit_to_int))

  (tag $e1_unit_to_unit)
  (tag $e2_int_to_unit (param i32))
  (tag $e3_int_to_int (param i32) (result i32))

  (global $i (mut i32) (i32.const 0))


  ;; Used for testing the passing of arguments to continuation function and returning values out of them
  (func $f1 (export "f1") (param $x i32) (result i32)
    (global.set  $i (i32.add (global.get $i) (local.get $x)))
    (suspend $e1_unit_to_unit)
    (i32.add (i32.const 2) (local.get $x)))

  ;; Used for testing case where no suspend happens at all
  (func $f2 (export "f2") (param $x i32) (result i32)
    (global.set  $i (i32.add (global.get $i) (local.get $x)))
    (i32.add (i32.const 2) (local.get $x)))

  ;; Same as $f1, but additionally passes payloads to and from handler
  (func $f3 (export "f3") (param $x i32) (result i32)
    (i32.add (local.get $x) (i32.const 1))
    (suspend $e3_int_to_int)
    ;; return x + value returned received back from $e3
    (i32.add  (local.get $x)))


  (func $test_case_1 (export "test_case_1") (result i32)
    ;; remove this eventually
    (global.set  $i (i32.const 0))
    (block $on_e1 (result (ref $res_unit_to_int))
      (resume $f1_ct (on $e1_unit_to_unit $on_e1) (i32.const 100) (cont.new $f1_ct (ref.func $f1)))
      ;; unreachable: we never intend to invoke the resumption when handling
      ;; $e1 invoked from $f2
      (unreachable))
    ;; after on_e1, stack: [resumption]
    (drop) ;; drop resumption
    (global.get $i))

  (func $test_case_2 (export "test_case_2") (result i32)
    ;; remove this eventually
    (global.set  $i (i32.const 0))
    ;;(local $finish_f3 (ref $res_unit_to_unit))
    (block $on_e1 (result (ref $res_unit_to_int))
      (resume $f1_ct (on $e1_unit_to_unit $on_e1) (i32.const 49) (cont.new $f1_ct (ref.func $f1)))
      (unreachable))
    ;; after on_e1, stack: [resumption]
    ;;(local.set $finish_f2)
    (resume $res_unit_to_int)
    ;; the resume above resumes execution of f2, which finishes without further suspends
    (i32.add (global.get $i)))

  (func $test_case_3 (export "test_case_3") (result i32)
    ;; remove this eventually
    (global.set  $i (i32.const 0))
    (resume $f2_ct (i32.const 49) (cont.new $f2_ct (ref.func $f2)))
    (i32.add (global.get $i)))


  (func $test_case_4 (export "test_case_4") (result i32)
    (local $k (ref $res_int_to_int))

    (block $on_e3 (result i32 (ref $res_int_to_int))
      (resume $f3_ct (on $e3_int_to_int $on_e3) (i32.const 49) (cont.new $f3_ct (ref.func $f3)))
      (unreachable))
    ;; after on_e3, expected stack: [50 resumption]
    (local.set $k)

    ;; add 1 to value 50 received from f6 via tag e3, thus passing 51 back to it
    (i32.add (i32.const 1))
    (resume $res_int_to_int (local.get $k))
    ;; expecting to get 49 (original argument to function) + 51 (passed above) back
    )

)

(assert_return (invoke "test_case_1") (i32.const 100))
(assert_return (invoke "test_case_2") (i32.const 100))
(assert_return (invoke "test_case_3") (i32.const 100))
(assert_return (invoke "test_case_4") (i32.const 100))
