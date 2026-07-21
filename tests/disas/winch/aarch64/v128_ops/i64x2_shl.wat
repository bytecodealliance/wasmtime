;;! target = "aarch64"
;;! test = "winch"

(module
  (memory 1)
  (func (export "run")
    (v128.store (i32.const 0)
      (i64x2.shl
        (v128.const i64x2 0x8000000000000000 0xFFFFFFFFFFFFFFFF)
        (i32.const 2)))))
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
;;       b.lo    #0x7c
;;   2c: mov     x9, x0
;;       sub     x28, x28, #0x10
;;       mov     sp, x28
;;       stur    x0, [x28, #8]
;;       stur    x1, [x28]
;;       mov     x0, #2
;;       ldr     q0, #0x80
;;       and     w0, w0, #0x3f
;;       dup     v31.2d, x0
;;       sshl    v0.2d, v0.2d, v31.2d
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
;;   7c: udf     #0xc11f
;;   80: udf     #0
;;   84: .byte   0x00, 0x00, 0x00, 0x80
;;   88: .byte   0xff, 0xff, 0xff, 0xff
;;   8c: .byte   0xff, 0xff, 0xff, 0xff
