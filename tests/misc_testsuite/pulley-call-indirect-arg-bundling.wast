;; The Pulley `call_indirect{1,2,3,4}` ops bundle the first four integer ABI
;; arguments into the indirect-call op; a fifth+ integer arg, and any float
;; arg, must still be passed the normal way. Exercise every count across that
;; boundary through a call_indirect and check the results are unchanged.

(module
  (type $t0 (func (result i32)))
  (type $t1 (func (param i32) (result i32)))
  (type $t4 (func (param i32 i32 i32 i32) (result i32)))
  (type $t6 (func (param i32 i32 i32 i32 i32 i32) (result i32)))
  (type $tf (func (param i32 f64 i32 f64 i32) (result f64)))

  (table 5 5 funcref)
  (elem (i32.const 0) $f0 $f1 $f4 $f6 $ff)

  (func $f0 (type $t0) (i32.const 100))
  (func $f1 (type $t1) (local.get 0))
  (func $f4 (type $t4)
    (i32.add (i32.add (local.get 0) (local.get 1))
             (i32.add (local.get 2) (local.get 3))))
  (func $f6 (type $t6)
    (i32.add
      (i32.add (i32.add (local.get 0) (local.get 1)) (i32.add (local.get 2) (local.get 3)))
      (i32.add (local.get 4) (local.get 5))))
  ;; Interleaved int/float args: the floats never occupy x0..x3, so the
  ;; bundling must skip them and still sum the integers + floats correctly.
  (func $ff (type $tf)
    (f64.add
      (f64.add (local.get 1) (local.get 3))
      (f64.convert_i32_s (i32.add (i32.add (local.get 0) (local.get 2)) (local.get 4)))))

  (func (export "c0") (result i32)
    (call_indirect (type $t0) (i32.const 0)))
  (func (export "c1") (param i32) (result i32)
    (call_indirect (type $t1) (local.get 0) (i32.const 1)))
  (func (export "c4") (param i32 i32 i32 i32) (result i32)
    (call_indirect (type $t4) (local.get 0) (local.get 1) (local.get 2) (local.get 3)
                   (i32.const 2)))
  (func (export "c6") (param i32 i32 i32 i32 i32 i32) (result i32)
    (call_indirect (type $t6) (local.get 0) (local.get 1) (local.get 2)
                   (local.get 3) (local.get 4) (local.get 5) (i32.const 3)))
  (func (export "cf") (param i32 f64 i32 f64 i32) (result f64)
    (call_indirect (type $tf) (local.get 0) (local.get 1) (local.get 2)
                   (local.get 3) (local.get 4) (i32.const 4))))

(assert_return (invoke "c0") (i32.const 100))
(assert_return (invoke "c1" (i32.const 42)) (i32.const 42))
(assert_return (invoke "c4" (i32.const 1) (i32.const 2) (i32.const 3) (i32.const 4)) (i32.const 10))
(assert_return (invoke "c6" (i32.const 1) (i32.const 2) (i32.const 3)
                            (i32.const 4) (i32.const 5) (i32.const 6)) (i32.const 21))
(assert_return (invoke "cf" (i32.const 1) (f64.const 2.5) (i32.const 3) (f64.const 4.5) (i32.const 5))
               (f64.const 16))
