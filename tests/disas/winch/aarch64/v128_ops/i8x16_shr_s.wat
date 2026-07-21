;;! target = "aarch64"
;;! test = "winch"

(module
  (memory 1)
  (func (export "run")
    (v128.store (i32.const 0)
      (i8x16.shr_s
        (v128.const i8x16 128 1 255 64 2 250 7 8 9 10 11 12 13 14 15 16)
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
;;       and     w0, w0, #7
;;       neg     x0, x0
;;       dup     v31.16b, w0
;;       sshl    v0.16b, v0.16b, v31.16b
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
;;   90: .byte   0x80, 0x01, 0xff, 0x40
;;   94: stlxrb  w7, w2, [x16]
;;   98: .byte   0x09, 0x0a, 0x0b, 0x0c
;;   9c: adr     x13, #0x1e25c
