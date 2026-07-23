;;! target = "aarch64"
;;! test = "winch"

(module
  (memory 1)
  (func (export "run")
    (v128.store (i32.const 0)
      (i8x16.sub_sat_u
        (v128.const i8x16 3 255 100 0 1 128 254 255 3 5 250 6 64 64 255 0)
        (v128.const i8x16 10 1 100 1 1 129 255 254 3 5 10 250 65 63 0 255)))))
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
;;       uqsub   v1.16b, v1.16b, v0.16b
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
;;   80: .byte   0x0a, 0x01, 0x64, 0x01
;;   84: .byte   0x01, 0x81, 0xff, 0xfe
;;   88: .byte   0x03, 0x05, 0x0a, 0xfa
;;   8c: .byte   0x41, 0x3f, 0x00, 0xff
;;   90: .byte   0x03, 0xff, 0x64, 0x00
;;   94: .byte   0x01, 0x80, 0xfe, 0xff
;;   98: .byte   0x03, 0x05, 0xfa, 0x06
;;   9c: .byte   0x40, 0x40, 0xff, 0x00
