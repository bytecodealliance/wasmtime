;;! target = "aarch64"
;;! test = "compile"

(module
  (func (param v128) (result v128)
    local.get 0
    i32x4.relaxed_trunc_f32x4_s
  )

  (func (param v128) (result v128)
    local.get 0
    i32x4.relaxed_trunc_f32x4_u
  )

  (func (param v128) (result v128)
    local.get 0
    i32x4.relaxed_trunc_f64x2_s_zero
  )

  (func (param v128) (result v128)
    local.get 0
    i32x4.relaxed_trunc_f64x2_u_zero
  )

  (func (param v128 v128) (result v128)
    local.get 0
    local.get 1
    i16x8.relaxed_dot_i8x16_i7x16_s
  )

  (func (param v128 v128 v128) (result v128)
    local.get 0
    local.get 1
    local.get 2
    i32x4.relaxed_dot_i8x16_i7x16_add_s
  )
)

;; wasm[0]::function[0]:
;;    0: stp     x29, x30, [sp, #-0x10]!
;;    4: mov     x29, sp
;;    8: fcvtzs  v0.4s, v0.4s
;;    c: ldp     x29, x30, [sp], #0x10
;;   10: ret
;;
;; wasm[0]::function[1]:
;;   20: stp     x29, x30, [sp, #-0x10]!
;;   24: mov     x29, sp
;;   28: fcvtzu  v0.4s, v0.4s
;;   2c: ldp     x29, x30, [sp], #0x10
;;   30: ret
;;
;; wasm[0]::function[2]:
;;   40: stp     x29, x30, [sp, #-0x10]!
;;   44: mov     x29, sp
;;   48: fcvtzs  v6.2d, v0.2d
;;   4c: sqxtn   v0.2s, v6.2d
;;   50: ldp     x29, x30, [sp], #0x10
;;   54: ret
;;
;; wasm[0]::function[3]:
;;   60: stp     x29, x30, [sp, #-0x10]!
;;   64: mov     x29, sp
;;   68: fcvtzu  v6.2d, v0.2d
;;   6c: uqxtn   v0.2s, v6.2d
;;   70: ldp     x29, x30, [sp], #0x10
;;   74: ret
;;
;; wasm[0]::function[4]:
;;   80: stp     x29, x30, [sp, #-0x10]!
;;   84: mov     x29, sp
;;   88: smull   v16.8h, v0.8b, v1.8b
;;   8c: smull2  v17.8h, v0.16b, v1.16b
;;   90: addp    v0.8h, v16.8h, v17.8h
;;   94: ldp     x29, x30, [sp], #0x10
;;   98: ret
;;
;; wasm[0]::function[5]:
;;   a0: stp     x29, x30, [sp, #-0x10]!
;;   a4: mov     x29, sp
;;   a8: smull   v19.8h, v0.8b, v1.8b
;;   ac: smull2  v20.8h, v0.16b, v1.16b
;;   b0: addp    v19.8h, v19.8h, v20.8h
;;   b4: saddlp  v19.4s, v19.8h
;;   b8: add     v0.4s, v19.4s, v2.4s
;;   bc: ldp     x29, x30, [sp], #0x10
;;   c0: ret
