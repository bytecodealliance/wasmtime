;;! nan_canonicalization = true

;; Scalar counterpart to simd/canonicalize-nan.wast.

(module
  (func (export "f32.add") (param f32 f32) (result f32)
    local.get 0
    local.get 1
    f32.add)
  (func (export "f32.sub") (param f32 f32) (result f32)
    local.get 0
    local.get 1
    f32.sub)
  (func (export "f32.mul") (param f32 f32) (result f32)
    local.get 0
    local.get 1
    f32.mul)
  (func (export "f32.div") (param f32 f32) (result f32)
    local.get 0
    local.get 1
    f32.div)
  (func (export "f32.min") (param f32 f32) (result f32)
    local.get 0
    local.get 1
    f32.min)
  (func (export "f32.max") (param f32 f32) (result f32)
    local.get 0
    local.get 1
    f32.max)
  (func (export "f32.sqrt") (param f32) (result f32)
    local.get 0
    f32.sqrt)
  (func (export "f32.ceil") (param f32) (result f32)
    local.get 0
    f32.ceil)
  (func (export "f32.floor") (param f32) (result f32)
    local.get 0
    f32.floor)
  (func (export "f32.trunc") (param f32) (result f32)
    local.get 0
    f32.trunc)
  (func (export "f32.nearest") (param f32) (result f32)
    local.get 0
    f32.nearest)
  (func (export "f64.div") (param f64 f64) (result f64)
    local.get 0
    local.get 1
    f64.div)
  (func (export "f64.sqrt") (param f64) (result f64)
    local.get 0
    f64.sqrt)

  (func (export "reinterpret-and-demote") (param i64) (result i32)
    local.get 0
    f64.reinterpret_i64
    f32.demote_f64
    i32.reinterpret_f32)
  (func (export "reinterpret-and-promote") (param i32) (result i64)
    local.get 0
    f32.reinterpret_i32
    f64.promote_f32
    i64.reinterpret_f64)

  ;; Expose raw bits of 0/0 to verify exact canonical NaN bit patterns.
  (func (export "f32.div-nan-bits") (result i32)
    f32.const 0
    f32.const 0
    f32.div
    i32.reinterpret_f32)
  (func (export "f64.div-nan-bits") (result i64)
    f64.const 0
    f64.const 0
    f64.div
    i64.reinterpret_f64)
)

;; Exact bit patterns: canonical f32 NaN = 0x7fc00000, f64 = 0x7ff8000000000000
(assert_return (invoke "f32.div-nan-bits") (i32.const 0x7fc00000))
(assert_return (invoke "f64.div-nan-bits") (i64.const 0x7ff8000000000000))

;; NaN-producing operations
(assert_return (invoke "f32.div" (f32.const 0) (f32.const 0)) (f32.const nan:0x400000))
(assert_return (invoke "f64.div" (f64.const 0) (f64.const 0)) (f64.const nan:0x8000000000000))
(assert_return (invoke "f32.sqrt" (f32.const -1)) (f32.const nan:0x400000))
(assert_return (invoke "f64.sqrt" (f64.const -1)) (f64.const nan:0x8000000000000))

;; NaN propagation through arithmetic
(assert_return (invoke "f32.add" (f32.const nan) (f32.const 1)) (f32.const nan:0x400000))
(assert_return (invoke "f32.sub" (f32.const nan) (f32.const 1)) (f32.const nan:0x400000))
(assert_return (invoke "f32.mul" (f32.const nan) (f32.const 1)) (f32.const nan:0x400000))
(assert_return (invoke "f32.min" (f32.const nan) (f32.const 1)) (f32.const nan:0x400000))
(assert_return (invoke "f32.max" (f32.const nan) (f32.const 1)) (f32.const nan:0x400000))

;; Rounding NaN
(assert_return (invoke "f32.ceil" (f32.const nan)) (f32.const nan:0x400000))
(assert_return (invoke "f32.floor" (f32.const nan)) (f32.const nan:0x400000))
(assert_return (invoke "f32.trunc" (f32.const nan)) (f32.const nan:0x400000))
(assert_return (invoke "f32.nearest" (f32.const nan)) (f32.const nan:0x400000))

;; Demote/promote with non-canonical NaN bit patterns
(assert_return (invoke "reinterpret-and-demote" (i64.const 0xfffefdfccccdcecf)) (i32.const 0x7fc00000))
(assert_return (invoke "reinterpret-and-promote" (i32.const 0xfffefdfc)) (i64.const 0x7ff8000000000000))

;; Normal values pass through unchanged
(assert_return (invoke "f32.add" (f32.const 1) (f32.const 2)) (f32.const 3))
(assert_return (invoke "f64.div" (f64.const 10) (f64.const 2)) (f64.const 5))
(assert_return (invoke "f32.sqrt" (f32.const 4)) (f32.const 2))
