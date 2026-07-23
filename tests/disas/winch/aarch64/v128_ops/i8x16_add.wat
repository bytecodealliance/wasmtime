;;! target = "aarch64"
;;! test = "winch"

(module
  (memory 1)
  (func (export "run")
    (v128.store (i32.const 0)
      (i8x16.add
        (v128.const i8x16 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16)
        (v128.const i8x16 10 10 10 10 10 10 10 10 20 20 20 20 20 20 20 20)))))
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
;;       add     v1.16b, v1.16b, v0.16b
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
;;   88: b       #0x5050d8
;;   8c: b       #0x5050dc
;;   90: subr    z1.b, p0/m, z1.b, z16.b
;;   94: stxrb   w7, w5, [x16]
;;   98: .byte   0x09, 0x0a, 0x0b, 0x0c
;;   9c: adr     x13, #0x1e25c
