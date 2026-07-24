;;! target = "aarch64"
;;! test = "winch"

(module
  (func (export "i8x16_bitmask") (param v128) (result i32)
        (i8x16.bitmask (local.get 0)))
)
;; wasm[0]::function[0]:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       str     x28, [sp, #-0x10]!
;;       mov     x28, sp
;;       ldur    x16, [x0, #8]
;;       ldur    x16, [x16, #0x18]
;;       mov     x17, #0
;;       movk    x17, #0x20
;;       add     x16, x16, x17
;;       cmp     sp, x16
;;       b.lo    #0x7c
;;   2c: mov     x9, x0
;;       sub     x28, x28, #0x20
;;       mov     sp, x28
;;       stur    x0, [x28, #0x18]
;;       stur    x1, [x28, #0x10]
;;       stur    q0, [x28]
;;       ldur    q0, [x28]
;;       sshr    v0.16b, v0.16b, #7
;;       ldr     q31, #0x80
;;       and     v0.16b, v0.16b, v31.16b
;;       ext     v31.16b, v0.16b, v0.16b, #8
;;       zip1    v0.16b, v0.16b, v31.16b
;;       addv    h0, v0.8h
;;       umov    w0, v0.h[0]
;;       add     x28, x28, #0x20
;;       mov     sp, x28
;;       mov     sp, x28
;;       ldr     x28, [sp], #0x10
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;   7c: udf     #0xc11f
;;   80: stxrb   w4, w1, [x16]
;;   84: .byte   0x10, 0x20, 0x40, 0x80
;;   88: stxrb   w4, w1, [x16]
;;   8c: .byte   0x10, 0x20, 0x40, 0x80
