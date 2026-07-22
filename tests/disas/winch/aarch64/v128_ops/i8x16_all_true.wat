;;! target = "aarch64"
;;! test = "winch"

(module
  (func (export "run") (result i32)
    (i8x16.all_true (v128.const i8x16 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16))))
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
;;       b.lo    #0x6c
;;   2c: mov     x9, x0
;;       sub     x28, x28, #0x10
;;       mov     sp, x28
;;       stur    x0, [x28, #8]
;;       stur    x1, [x28]
;;       ldr     q0, #0x70
;;       uminv   b31, v0.16b
;;       mov     x0, v31.d[0]
;;       cmp     x0, #0
;;       cset    x0, ne
;;       add     x28, x28, #0x10
;;       mov     sp, x28
;;       mov     sp, x28
;;       ldr     x28, [sp], #0x10
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;   6c: udf     #0xc11f
;;   70: subr    z1.b, p0/m, z1.b, z16.b
;;   74: stxrb   w7, w5, [x16]
;;   78: .byte   0x09, 0x0a, 0x0b, 0x0c
;;   7c: adr     x13, #0x1e23c
