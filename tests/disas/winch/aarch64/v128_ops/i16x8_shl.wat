;;! target = "aarch64"
;;! test = "winch"

(module
  (memory 1)
  (func (export "run")
    (v128.store (i32.const 0)
      (i16x8.shl
        (v128.const i16x8 32768 1 65535 16384 2 3 4 5)
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
;;       and     w0, w0, #0xf
;;       dup     v31.8h, w0
;;       sshl    v0.8h, v0.8h, v31.8h
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
;;   80: .byte   0x00, 0x80, 0x01, 0x00
;;   84: .byte   0xff, 0xff, 0x00, 0x40
;;   88: .byte   0x02, 0x00, 0x03, 0x00
;;   8c: .byte   0x04, 0x00, 0x05, 0x00
