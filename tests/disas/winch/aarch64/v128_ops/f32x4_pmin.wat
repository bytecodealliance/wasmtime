;;! target = "aarch64"
;;! test = "winch"

(module
  (memory (data "\00\00\00\00\00\00\00\00\00\00\00\00\00\00\a0\7f"))

  (func (export "f32x4_pmin") (param v128) (param v128) (result v128)
        (f32x4.pmin (local.get 0) (local.get 1)))
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
;;       b.lo    #0x78
;;   2c: mov     x9, x0
;;       sub     x28, x28, #0x30
;;       mov     sp, x28
;;       stur    x0, [x28, #0x28]
;;       stur    x1, [x28, #0x20]
;;       stur    q0, [x28, #0x10]
;;       stur    q1, [x28]
;;       ldur    q0, [x28]
;;       ldur    q1, [x28, #0x10]
;;       fcmgt   v31.4s, v1.4s, v0.4s
;;       bsl     v31.16b, v0.16b, v1.16b
;;       mov     v1.16b, v31.16b
;;       mov     v0.16b, v1.16b
;;       add     x28, x28, #0x30
;;       mov     sp, x28
;;       mov     sp, x28
;;       ldr     x28, [sp], #0x10
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;   78: udf     #0xc11f
