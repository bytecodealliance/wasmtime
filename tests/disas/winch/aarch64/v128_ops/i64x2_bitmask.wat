;;! target = "aarch64"
;;! test = "winch"

(module
  (func (export "i64x2_bitmask") (param v128) (result i32)
        (i64x2.bitmask (local.get 0)))
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
;;       b.lo    #0x78
;;   2c: mov     x9, x0
;;       sub     x28, x28, #0x20
;;       mov     sp, x28
;;       stur    x0, [x28, #0x18]
;;       stur    x1, [x28, #0x10]
;;       stur    q0, [x28]
;;       ldur    q0, [x28]
;;       mov     x16, v0.d[1]
;;       mov     x0, v0.d[0]
;;       lsr     x16, x16, #0x3f
;;       lsr     x0, x0, #0x3f
;;       lsl     x16, x16, #1
;;       add     x0, x0, x16, uxtx
;;       add     x28, x28, #0x20
;;       mov     sp, x28
;;       mov     sp, x28
;;       ldr     x28, [sp], #0x10
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;   78: udf     #0xc11f
