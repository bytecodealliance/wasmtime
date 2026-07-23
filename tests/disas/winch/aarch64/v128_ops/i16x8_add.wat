;;! target = "aarch64"
;;! test = "winch"

(module
  (memory 1)
  (func (export "run")
    (v128.store (i32.const 0)
      (i16x8.add
        (v128.const i16x8 100 200 300 400 500 600 700 800)
        (v128.const i16x8 1 2 3 4 5 6 7 8)))))
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
;;       add     v1.8h, v1.8h, v0.8h
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
;;   80: .byte   0x01, 0x00, 0x02, 0x00
;;   84: .byte   0x03, 0x00, 0x04, 0x00
;;   88: .byte   0x05, 0x00, 0x06, 0x00
;;   8c: .byte   0x07, 0x00, 0x08, 0x00
;;   90: .byte   0x64, 0x00, 0xc8, 0x00
;;   94: .byte   0x2c, 0x01, 0x90, 0x01
;;   98: .byte   0xf4, 0x01, 0x58, 0x02
;;   9c: .byte   0xbc, 0x02, 0x20, 0x03
