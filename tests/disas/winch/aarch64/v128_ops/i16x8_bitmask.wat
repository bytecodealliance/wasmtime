;;! target = "aarch64"
;;! test = "winch"

(module
  (func (export "i16x8_bitmask") (param v128) (result i32)
        (i16x8.bitmask (local.get 0)))
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
;;       b.lo    #0x74
;;   2c: mov     x9, x0
;;       sub     x28, x28, #0x20
;;       mov     sp, x28
;;       stur    x0, [x28, #0x18]
;;       stur    x1, [x28, #0x10]
;;       stur    q0, [x28]
;;       ldur    q0, [x28]
;;       sshr    v0.8h, v0.8h, #0xf
;;       ldr     q31, #0x80
;;       and     v0.16b, v0.16b, v31.16b
;;       addv    h0, v0.8h
;;       umov    w0, v0.h[0]
;;       add     x28, x28, #0x20
;;       mov     sp, x28
;;       mov     sp, x28
;;       ldr     x28, [sp], #0x10
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;   74: udf     #0xc11f
;;   78: udf     #0
;;   7c: udf     #0
;;   80: .byte   0x01, 0x00, 0x02, 0x00
;;   84: .byte   0x04, 0x00, 0x08, 0x00
;;   88: .byte   0x10, 0x00, 0x20, 0x00
;;   8c: .byte   0x40, 0x00, 0x80, 0x00
