;;! target = "aarch64"
;;! test = "winch"

(module
  (memory 1)
  (func (export "run")
    (v128.store (i32.const 0)
      (i8x16.le_s
        (v128.const i8x16 128 1 255 5 100 200 7 0 64 32 16 8 4 2 1 0)
        (v128.const i8x16 127 1 0 9 100 100 8 255 64 33 15 8 5 1 2 0)))))
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
;;       cmge    v1.16b, v0.16b, v1.16b
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
;;   80: .byte   0x7f, 0x01, 0x00, 0x09
;;   84: .byte   0x64, 0x64, 0x08, 0xff
;;   88: stxrb   w15, w0, [x10]
;;   8c: .byte   0x05, 0x01, 0x02, 0x00
;;   90: .byte   0x80, 0x01, 0xff, 0x05
;;   94: .byte   0x64, 0xc8, 0x07, 0x00
;;   98: stxrb   w16, w0, [x2]
;;   9c: .byte   0x04, 0x02, 0x01, 0x00
