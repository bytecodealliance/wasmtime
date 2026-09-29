;;! exceptions = true
;;! reference_types = true
;;! bulk_memory = true
;;! simd = true

;; Cover several payload shapes: empty, scalar, mixed, vector, and reference
;; values.

(module
  (tag $empty)
  (tag $one (param i32))
  (tag $mixed (param i32 i64 f64))

  (func (export "throw-empty") (throw $empty))
  (func (export "throw-one") (throw $one (i32.const 42)))
  (func (export "throw-mixed") (param i32 i64 f64)
    (throw $mixed (local.get 0) (local.get 1) (local.get 2)))

  ;; `v128` payloads occupy a full slot.
  (tag $vec (param i32 v128 f64))
  (func (export "throw-vec")
    (throw $vec (i32.const 1) (v128.const i64x2 2 3) (f64.const 4))))

(assert_exception (invoke "throw-empty"))
(assert_exception (invoke "throw-one"))
(assert_exception
  (invoke "throw-mixed" (i32.const 1) (i64.const 2) (f64.const 3)))
(assert_exception (invoke "throw-vec"))

;; A reference payload remains live across the exception allocation.
(module
  (tag $eref (param externref))
  (func (export "throw-ref") (param externref)
    (throw $eref (local.get 0))))

(assert_exception (invoke "throw-ref" (ref.extern 5)))

;; A function reference is interned before it is stored in the GC heap.
(module
  (tag $fref (param funcref))
  (func $f)
  (elem declare func $f)
  (func (export "throw-funcref")
    (throw $fref (ref.func $f))))

(assert_exception (invoke "throw-funcref"))
