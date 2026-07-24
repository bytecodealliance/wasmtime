;;! target = "aarch64"
;;! test = "winch"

(module
  (func (export "i8x16_shuffle") (param v128) (param v128) (result v128)
        (i8x16.shuffle 0 17 2 19 4 21 6 23 8 25 10 27 12 29 14 31
          (local.get 0) (local.get 1)))
)
;; wasm[0]::function[0]:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       str     x28, [sp, #-0x10]!
;;       mov     x28, sp
;;       ldur    x16, [x0, #8]
;;       ldur    x16, [x16, #0x18]
;;       mov     x17, #0
;;       movk    x17, #0x30
;;       add     x16, x16, x17
;;       cmp     sp, x16
;;       b.lo    #0x7c
;;   2c: mov     x9, x0
;;       sub     x28, x28, #0x30
;;       mov     sp, x28
;;       stur    x0, [x28, #0x28]
;;       stur    x1, [x28, #0x20]
;;       stur    q0, [x28, #0x10]
;;       stur    q1, [x28]
;;       ldur    q0, [x28]
;;       ldur    q1, [x28, #0x10]
;;       ldr     q31, #0x80
;;       tbl     v1.16b, {v1.16b}, v31.16b
;;       ldr     q31, #0x90
;;       tbx     v1.16b, {v0.16b}, v31.16b
;;       mov     v0.16b, v1.16b
;;       add     x28, x28, #0x30
;;       mov     sp, x28
;;       mov     sp, x28
;;       ldr     x28, [sp], #0x10
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;   7c: udf     #0xc11f
;;   80: sbfx    w0, w8, #2, #3
;;   84: b       #0xfffffffffc185494
;;   88: madd    w8, w8, w10, w6
;;   8c: fmadd   s12, s8, s14, s7
;;   90: .byte   0xf0, 0x01, 0xf2, 0x03
;;   94: .byte   0xf4, 0x05, 0xf6, 0x07
;;   98: .byte   0xf8, 0x09, 0xfa, 0x0b
;;   9c: .byte   0xfc, 0x0d, 0xfe, 0x0f
