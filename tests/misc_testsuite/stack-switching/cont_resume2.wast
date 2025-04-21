;;! stack_switching = true
;; This test requires the following to work:
;; 1. Passing arguments to tags and receiving values back at suspend sites
;; 2. Passing values to a continuation obtained from a handler
;; 3. Receving values once a resumed continuation returns oridinarly

;; Not tested: Passing values when resuming a continuation obtained from
;; cont.new rather than a handler

;; TODO(frank-emrich) Replace this with more fine-grained tests in the future

(module

  (type $int_to_int (func (param i32) (result i32)))

  (type $cont_int_to_int (cont $int_to_int))


  (type $g_type (func (result i32)))
  (type $g_ct (cont $g_type))


  (tag $e0 (param i32) (result i32)) ;; never actually invoked
  (tag $e1 (param i32) (result i32))
  (tag $e2 (param i32) (result i32))
  (tag $e3 (param i32) (result i32)) ;; never actually invoked


  (func $g (result i32)
    (suspend $e1 (i32.const 42))
    (suspend $e2) ;; passes value obtained from doing $e on to $f
    (i32.add (i32.const 21)))
  (elem declare func $g)

  (func $f (export "f") (result i32)
    (local $c (ref $cont_int_to_int))
    (block $on_e0_e2_e3 (result i32 (ref $cont_int_to_int))
      (block $on_e1 (result i32 (ref $cont_int_to_int))
        ;; We know that $e0, e2, e3 won't actually be performed here, but add a handler
        ;; to test the switching logic
        (resume $g_ct
          (on $e0 $on_e0_e2_e3)
          (on $e1 $on_e1)
          (on $e2 $on_e0_e2_e3)
          (on $e3 $on_e0_e2_e3)
          (cont.new $g_ct (ref.func $g)))
        (unreachable))
      ;; after $on_e1
      (local.set $c)
      ;; stack now contains the value that $g passed to $e1, we manipulate it
      (i32.add (i32.const 13))
      (local.get $c)
      (resume $cont_int_to_int (on $e2 $on_e0_e2_e3))
      (unreachable))
    ;; after $on_e0_e2_e3
    ;; stack contains value that $g passed to $e2 and continuation
    ;; We manipulate the value again before resuming the continuation
    (local.set $c)
    (i32.add (i32.const 24))
    (local.get $c)
    (resume $cont_int_to_int)
    ))


(assert_return (invoke "f") (i32.const 100))
