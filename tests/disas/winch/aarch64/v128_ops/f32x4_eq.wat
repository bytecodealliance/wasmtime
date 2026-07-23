;;! target = "aarch64"
;;! test = "winch"

(module
  (memory 1)
  (func (export "run")
    (v128.store (i32.const 0)
      (f32x4.eq
        (v128.const f32x4 1.5 nan -0x0p0 100)
        (v128.const f32x4 1.5 2 0 -3)))))
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
;;       fcmeq   v1.4s, v1.4s, v0.4s
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
;;   80: .byte   0x00, 0x00, 0xc0, 0x3f
;;   84: .byte   0x00, 0x00, 0x00, 0x40
;;   88: udf     #0
;;   8c: mov     za0h.h[w12, 0], p0/m, z0.h
;;   90: .byte   0x00, 0x00, 0xc0, 0x3f
;;   94: .byte   0x00, 0x00, 0xc0, 0x7f
;;   98: .byte   0x00, 0x00, 0x00, 0x80
;;   9c: .byte   0x00, 0x00, 0xc8, 0x42
