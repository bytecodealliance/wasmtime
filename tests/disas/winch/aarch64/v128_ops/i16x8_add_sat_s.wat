;;! target = "aarch64"
;;! test = "winch"

(module
  (memory 1)
  (func (export "run")
    (v128.store (i32.const 0)
      (i16x8.add_sat_s
        (v128.const i16x8 32767 -32768 1000 -1000 0 1 32767 -1)
        (v128.const i16x8 1 -1 1000 -1000 0 1 -1 32767)))))
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
;;       sqadd   v1.8h, v1.8h, v0.8h
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
;;   80: .byte   0x01, 0x00, 0xff, 0xff
;;   84: stur    d8, [sp, #-0x80]
;;   88: .byte   0x00, 0x00, 0x01, 0x00
;;   8c: .byte   0xff, 0xff, 0xff, 0x7f
;;   90: .byte   0xff, 0x7f, 0x00, 0x80
;;   94: stur    d8, [sp, #-0x80]
;;   98: .byte   0x00, 0x00, 0x01, 0x00
;;   9c: .byte   0xff, 0x7f, 0xff, 0xff
