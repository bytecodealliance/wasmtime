;;! simd = true

(module
    (func (export "f32x4_lane0") (result v128)
        v128.const f32x4 2 3 4 5
        f32.const 1
        f32x4.replace_lane 0
    )

    (func (export "f32x4_lane1") (result v128)
        v128.const f32x4 2 3 4 5
        f32.const 1
        f32x4.replace_lane 1
    )

    (func (export "f64x2_lane0") (result v128)
        v128.const f64x2 2 3
        f64.const 1
        f64x2.replace_lane 0
    )

    (func (export "f64x2_lane1") (result v128)
        v128.const f64x2 2 3
        f64.const 1
        f64x2.replace_lane 1
    )
)

(assert_return (invoke "f32x4_lane0") (v128.const f32x4 1 3 4 5))
(assert_return (invoke "f32x4_lane1") (v128.const f32x4 2 1 4 5))
(assert_return (invoke "f64x2_lane0") (v128.const f64x2 1 3))
(assert_return (invoke "f64x2_lane1") (v128.const f64x2 2 1))
