;;! target = "aarch64"
;;! test = "winch"

(module
  (memory 1)
  (func (export "run")
    (v128.store (i32.const 0)
      (i32x4.shr_u
        (v128.const i32x4 0x80000000 1 0xFFFFFFFF 7)
        (i32.const 3)))))
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
;;       b.lo    #0x80
;;   2c: mov     x9, x0
;;       sub     x28, x28, #0x10
;;       mov     sp, x28
;;       stur    x0, [x28, #8]
;;       stur    x1, [x28]
;;       mov     x0, #3
;;       ldr     q0, #0x90
;;       and     w0, w0, #0x1f
;;       neg     x0, x0
;;       dup     v31.4s, w0
;;       ushl    v0.4s, v0.4s, v31.4s
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
;;   80: udf     #0xc11f
;;   84: udf     #0
;;   88: udf     #0
;;   8c: udf     #0
;;   90: .byte   0x00, 0x00, 0x00, 0x80
;;   94: udf     #1
;;   98: .byte   0xff, 0xff, 0xff, 0xff
;;   9c: udf     #7
