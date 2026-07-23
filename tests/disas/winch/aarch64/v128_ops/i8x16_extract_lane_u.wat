;;! target = "aarch64"
;;! test = "winch"

(module
  (func (export "run") (result i32)
    (i8x16.extract_lane_u 0 (v128.const i8x16 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25))))
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
;;       b.lo    #0x60
;;   2c: mov     x9, x0
;;       sub     x28, x28, #0x10
;;       mov     sp, x28
;;       stur    x0, [x28, #8]
;;       stur    x1, [x28]
;;       ldr     q0, #0x70
;;       umov    w0, v0.b[0]
;;       add     x28, x28, #0x10
;;       mov     sp, x28
;;       mov     sp, x28
;;       ldr     x28, [sp], #0x10
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;   60: udf     #0xc11f
;;   64: udf     #0
;;   68: udf     #0
;;   6c: udf     #0
;;   70: .byte   0x0a, 0x0b, 0x0c, 0x0d
;;   74: add     w14, w24, #0x403
;;   78: b       #0x4504cc0
;;   7c: .byte   0x16, 0x17, 0x18, 0x19
