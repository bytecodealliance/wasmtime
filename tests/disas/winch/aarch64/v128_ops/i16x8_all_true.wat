;;! target = "aarch64"
;;! test = "winch"

(module
  (func (export "run") (result i32)
    (i16x8.all_true (v128.const i16x8 1 2 3 4 5 6 7 0))))
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
;;       uminv   h31, v0.8h
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
;;   70: .byte   0x01, 0x00, 0x02, 0x00
;;   74: .byte   0x03, 0x00, 0x04, 0x00
;;   78: .byte   0x05, 0x00, 0x06, 0x00
;;   7c: udf     #7
