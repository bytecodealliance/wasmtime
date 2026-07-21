;;! target = "aarch64"
;;! test = "winch"

(module
  (memory 1)
  (func (export "run")
    (v128.store (i32.const 0)
      (i8x16.add_sat_s
        (v128.const i8x16 127 -128 100 -100 0 1 -1 50 127 -128 5 -5 64 -64 127 -1)
        (v128.const i8x16 1 -1 100 -100 0 1 -1 50 127 -128 5 -5 64 -64 -1 127)))))
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
;;       sqadd   v1.16b, v1.16b, v0.16b
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
;;   80: ldr     q1, #0xca060
;;   84: .byte   0x00, 0x01, 0xff, 0x32
;;   88: .byte   0x7f, 0x80, 0x05, 0xfb
;;   8c: .byte   0x40, 0xc0, 0xff, 0x7f
;;   90: ldr     q31, #0xc909c
;;   94: .byte   0x00, 0x01, 0xff, 0x32
;;   98: .byte   0x7f, 0x80, 0x05, 0xfb
;;   9c: .byte   0x40, 0xc0, 0x7f, 0xff
