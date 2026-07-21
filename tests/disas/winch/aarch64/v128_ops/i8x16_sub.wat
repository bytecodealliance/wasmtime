;;! target = "aarch64"
;;! test = "winch"

(module
  (memory 1)
  (func (export "run")
    (v128.store (i32.const 0)
      (i8x16.sub
        (v128.const i8x16 50 60 70 80 90 100 110 120 5 10 15 20 25 30 35 40)
        (v128.const i8x16 10 10 10 10 10 10 10 10 10 20 30 40 50 60 70 80)))))
;; wasm[0]::function[0]:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       str     x28, [sp, #-0x10]!
;;       mov     x28, sp
;;       ldur    x16, [x0, #8]
;;       ldur    x16, [x16, #0x18]
;;       mov     x17, #0
;;       movk    x17, #0x10
;;       add     x16, x16, x17
;;       cmp     sp, x16
;;       b.lo    #0x74
;;   2c: mov     x9, x0
;;       sub     x28, x28, #0x10
;;       mov     sp, x28
;;       stur    x0, [x28, #8]
;;       stur    x1, [x28]
;;       ldr     q0, #0x80
;;       ldr     q1, #0x90
;;       sub     v1.16b, v1.16b, v0.16b
;;       mov     x0, #0
;;       ldur    x1, [x9, #0x38]
;;       add     x1, x1, w0, uxtw
;;       stur    q1, [x1]
;;       add     x28, x28, #0x10
;;       mov     sp, x28
;;       mov     sp, x28
;;       ldr     x28, [sp], #0x10
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;   74: udf     #0xc11f
;;   78: udf     #0
;;   7c: udf     #0
;;   80: and     w10, w16, w10, lsl #2
;;   84: and     w10, w16, w10, lsl #2
;;   88: stnp    w10, w5, [x0, #0xf0]
;;   8c: adr     x18, #0x8c812
;;   90: adr     x18, #0x8c816
;;   94: .byte   0x5a, 0x64, 0x6e, 0x78
;;   98: b       #0x3c28ac
;;   9c: stnp    w25, w7, [x16, #-0xe8]
