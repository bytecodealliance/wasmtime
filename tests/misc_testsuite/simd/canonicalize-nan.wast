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
