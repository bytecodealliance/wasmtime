;;! simd = true

(; See issue https://github.com/bytecodealliance/wasmtime/issues/3327 ;)

(module
  (func $v128_not (export "v128_not") (result v128)
    v128.const f32x4 0 0 0 0
    f32x4.abs
    v128.not)
)

(assert_return (invoke "v128_not") (v128.const i32x4 -1 -1 -1 -1))

;; from #3327
(module
  (func (result i32)
    v128.const i32x4 0xffffffff 0x80bfffff 0x80bf0a0a 0x80bf0a0a
    f64x2.promote_low_f32x4
    v128.not
    v128.not
    v128.not
    v128.not
    v128.not
    v128.not
    v128.not
    v128.const i32x4 0 0 0 0
    f64x2.gt
    v128.not
    i64x2.bitmask)
  (export "" (func 0)))
;; the f64x2.promote_low_f32x4 operation may or may not preserve the sign bit
;; on the NaN in the first operation. This leads to one of two results depending
;; on how platforms propagate NaN bits.
(assert_return (invoke "") (either (i32.const 0) (i32.const 1)))

;; from #3327
(module
  (type (func (param i32) (result i32)))
  (func (type 0) (param i32) (result i32)
    local.get 0
    i32x4.splat
    f64x2.abs
    v128.not
    i64x2.bitmask)
  (export "1" (func 0)))
(assert_return (invoke "1" (i32.const 0)) (i32.const 3))

(module
  (type (;0;) (func (result v128)))
  (func (;0;) (type 0) (result v128)
      v128.const i32x4 0x733c3e67 0x3c3e6776 0x3e677673 0x6776733c
      i64x2.abs
      i64x2.bitmask
      i8x16.splat
      v128.const i32x4 0x733c3e67 0x3c3e6776 0x3e677673 0x6776733c
      i64x2.ge_s
      f32x4.floor
      v128.not
      i16x8.extadd_pairwise_i8x16_u)
  (export "x" (func 0)))
(assert_return (invoke "x") (v128.const i32x4 0x01fe01fe 0x01fe01fe 0x01fe01fe 0x01fe01fe))
