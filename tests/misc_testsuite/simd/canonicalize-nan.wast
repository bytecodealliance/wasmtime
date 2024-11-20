;;! nan_canonicalization = true
;;! simd = true

;; This *.wast test should be run with `cranelift_nan_canonicalization` set to
;; `true` in `wast.rs`

(module
  (func (export "f32x4.floor") (param v128) (result v128)
    local.get 0
    f32x4.floor)
  (func (export "f32x4.nearest") (param v128) (result v128)
    local.get 0
    f32x4.nearest)
  (func (export "f32x4.sqrt") (param v128) (result v128)
    local.get 0
    f32x4.sqrt)
  (func (export "f32x4.trunc") (param v128) (result v128)
    local.get 0
    f32x4.trunc)
  (func (export "f32x4.ceil") (param v128) (result v128)
    local.get 0
    f32x4.ceil)

  (func (export "f64x2.floor") (param v128) (result v128)
    local.get 0
    f64x2.floor)
  (func (export "f64x2.nearest") (param v128) (result v128)
    local.get 0
    f64x2.nearest)
  (func (export "f64x2.sqrt") (param v128) (result v128)
    local.get 0
    f64x2.sqrt)
  (func (export "f64x2.trunc") (param v128) (result v128)
    local.get 0
    f64x2.trunc)
  (func (export "f64x2.ceil") (param v128) (result v128)
    local.get 0
    f64x2.ceil)

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

  (func (export "copysign-and-demote") (param f64) (result f32)
    local.get 0
    f64.const -0x1
    f64.copysign
    f32.demote_f64)

  (func (export "copysign-and-promote") (param f32) (result f64)
    local.get 0
    f32.const -0x1
    f32.copysign
    f64.promote_f32)

  (func (export "f32x4.demote_f64x2_zero") (param v128) (result v128)
    local.get 0
    f32x4.demote_f64x2_zero)

  (func (export "f64x2.promote_low_f32x4") (param v128) (result v128)
    local.get 0
    f64x2.promote_low_f32x4)
)

(assert_return (invoke "f32x4.floor" (v128.const f32x4 1 -2.2 3.4 nan))
               (v128.const f32x4 1 -3 3 nan))
(assert_return (invoke "f32x4.nearest" (v128.const f32x4 1 -2.2 3.4 nan))
               (v128.const f32x4 1 -2 3 nan))
(assert_return (invoke "f32x4.sqrt" (v128.const f32x4 1 4 -1 nan))
               (v128.const f32x4 1 2 nan nan))
(assert_return (invoke "f32x4.trunc" (v128.const f32x4 1 -2.2 3.4 nan))
               (v128.const f32x4 1 -2 3 nan))
(assert_return (invoke "f32x4.ceil" (v128.const f32x4 1 -2.2 3.4 nan))
               (v128.const f32x4 1 -2 4 nan))

(assert_return (invoke "f64x2.floor" (v128.const f64x2 -2.2 nan))
               (v128.const f64x2 -3 nan))
(assert_return (invoke "f64x2.nearest" (v128.const f64x2 -2.2 nan))
               (v128.const f64x2 -2 nan))
(assert_return (invoke "f64x2.sqrt" (v128.const f64x2 4 nan))
               (v128.const f64x2 2 nan))
(assert_return (invoke "f64x2.trunc" (v128.const f64x2 3.4 nan))
               (v128.const f64x2 3 nan))
(assert_return (invoke "f64x2.ceil" (v128.const f64x2 3.4 nan))
               (v128.const f64x2 4 nan))

(assert_return (invoke "reinterpret-and-demote" (i64.const 0xfffefdfccccdcecf))
               (i32.const 0x7fc00000))
(assert_return (invoke "reinterpret-and-promote" (i32.const 0xfffefdfc))
               (i64.const 0x7ff8000000000000))
(assert_return (invoke "copysign-and-demote" (f64.const nan))
               (f32.const nan:0x7fc00000))
(assert_return (invoke "copysign-and-promote" (f32.const nan))
               (f64.const nan:0x7ff8000000000000))

(assert_return (invoke "f32x4.demote_f64x2_zero"
               (v128.const i64x2 0xfffefdfccccdcecf 0xfffefdfccccdcecf))
               (v128.const f32x4 nan:0x7fc00000 nan:0x7fc00000 0 0))

(assert_return (invoke "f64x2.promote_low_f32x4"
               (v128.const i32x4 0xfffefdfc 0xfffefdfc 0 0))
               (v128.const f64x2 nan:0x7ff8000000000000 nan:0x7ff8000000000000))
