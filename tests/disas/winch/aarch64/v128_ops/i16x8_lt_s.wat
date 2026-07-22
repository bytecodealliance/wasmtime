;;! target = "aarch64"
;;! test = "winch"

(module
  (memory 1)
  (func (export "run")
    (v128.store (i32.const 0)
      (i16x8.lt_s
        (v128.const i16x8 32768 1 65535 5 1000 2000 7 0)
        (v128.const i16x8 32767 1 0 9 1000 1999 8 65535)))))
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
;;       cmgt    v1.8h, v0.8h, v1.8h
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
;;   80: .byte   0xff, 0x7f, 0x01, 0x00
;;   84: .byte   0x00, 0x00, 0x09, 0x00
;;   88: .byte   0xe8, 0x03, 0xcf, 0x07
;;   8c: .byte   0x08, 0x00, 0xff, 0xff
;;   90: .byte   0x00, 0x80, 0x01, 0x00
;;   94: .byte   0xff, 0xff, 0x05, 0x00
;;   98: .byte   0xe8, 0x03, 0xd0, 0x07
;;   9c: udf     #7
