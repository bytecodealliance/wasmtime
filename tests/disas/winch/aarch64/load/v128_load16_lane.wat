;;! target = "aarch64"
;;! test = "winch"

(module
  (memory 1 1)
  (func (export "_start") (result v128)
        (v128.load16_lane
          1 (i32.const 0) (v128.const i64x2 0xFFFFFFFFFFFFFFFF 0xFFFFFFFFFFFFFFFF)
          )))
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
;;       b.lo    #0x70
;;   2c: mov     x9, x0
;;       sub     x28, x28, #0x10
;;       mov     sp, x28
;;       stur    x0, [x28, #8]
;;       stur    x1, [x28]
;;       ldr     q0, #0x80
;;       mov     x0, #0
;;       ldur    x1, [x9, #0x38]
;;       add     x1, x1, w0, uxtw
;;       ldurh   w16, [x1]
;;       mov     v0.h[1], w16
;;       add     x28, x28, #0x10
;;       mov     sp, x28
;;       mov     sp, x28
;;       ldr     x28, [sp], #0x10
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;   70: udf     #0xc11f
;;   74: udf     #0
;;   78: udf     #0
;;   7c: udf     #0
;;   80: .byte   0xff, 0xff, 0xff, 0xff
;;   84: .byte   0xff, 0xff, 0xff, 0xff
;;   88: .byte   0xff, 0xff, 0xff, 0xff
;;   8c: .byte   0xff, 0xff, 0xff, 0xff
