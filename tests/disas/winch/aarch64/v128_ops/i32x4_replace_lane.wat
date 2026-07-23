;;! target = "aarch64"
;;! test = "winch"

(module
  (memory 1)
  (func (export "run")
    (v128.store (i32.const 0)
      (i32x4.replace_lane 3 (v128.const i8x16 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25) (i32.const -1)))))
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
;;       mov     x16, #0xffffffff
;;       mov     v0.s[3], w16
;;       mov     x0, #0
;;       ldur    x1, [x9, #0x38]
;;       add     x1, x1, w0, uxtw
;;       stur    q0, [x1]
;;       add     x28, x28, #0x10
;;       mov     sp, x28
;;       mov     sp, x28
;;       ldr     x28, [sp], #0x10
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;   74: udf     #0xc11f
;;   78: udf     #0
;;   7c: udf     #0
;;   80: .byte   0x0a, 0x0b, 0x0c, 0x0d
;;   84: add     w14, w24, #0x403
;;   88: b       #0x4504cd0
;;   8c: .byte   0x16, 0x17, 0x18, 0x19
