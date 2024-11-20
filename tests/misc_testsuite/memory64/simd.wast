;;! memory64 = true
;;! simd = true

;; make sure everything codegens correctly and has no cranelift verifier errors
(module
  (memory i64 1)
  (func (export "run")
    i64.const 0 v128.load drop
    i64.const 0 v128.load8x8_s drop
    i64.const 0 v128.load8x8_u drop
    i64.const 0 v128.load16x4_s drop
    i64.const 0 v128.load16x4_u drop
    i64.const 0 v128.load32x2_s drop
    i64.const 0 v128.load32x2_u drop
    i64.const 0 v128.load8_splat drop
    i64.const 0 v128.load16_splat drop
    i64.const 0 v128.load32_splat drop
    i64.const 0 v128.load64_splat drop
    i64.const 0 i32.const 0 i8x16.splat v128.store
    i64.const 0 i32.const 0 i8x16.splat v128.store8_lane 0
    i64.const 0 i32.const 0 i8x16.splat v128.store16_lane 0
    i64.const 0 i32.const 0 i8x16.splat v128.store32_lane 0
    i64.const 0 i32.const 0 i8x16.splat v128.store64_lane 0
    i64.const 0 i32.const 0 i8x16.splat v128.load8_lane 0 drop
    i64.const 0 i32.const 0 i8x16.splat v128.load16_lane 0 drop
    i64.const 0 i32.const 0 i8x16.splat v128.load32_lane 0 drop
    i64.const 0 i32.const 0 i8x16.splat v128.load64_lane 0 drop
    i64.const 0 v128.load32_zero drop
    i64.const 0 v128.load64_zero drop
  )
)
(assert_return (invoke "run"))
