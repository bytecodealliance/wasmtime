;;! target = "aarch64"
;;! test = "winch"

(module
  (memory 1)
  (func (export "run")
    (v128.store (i32.const 0)
      (v128.bitselect
        (v128.const i64x2 1 2)
        (v128.const i64x2 3 4)
        (v128.const i64x2 -1 0)))))
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
;;       ldr     q0, #0x80
;;       ldr     q1, #0x90
;;       ldr     q2, #0xa0
;;       mov     v3.16b, v0.16b
;;       bsl     v3.16b, v2.16b, v1.16b
;;       mov     x0, #0
;;       ldur    x1, [x9, #0x38]
;;       add     x1, x1, w0, uxtw
;;       stur    q3, [x1]
;;       add     x28, x28, #0x10
;;       mov     sp, x28
;;       mov     sp, x28
;;       ldr     x28, [sp], #0x10
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;   7c: udf     #0xc11f
;;   80: .byte   0xff, 0xff, 0xff, 0xff
;;   84: .byte   0xff, 0xff, 0xff, 0xff
;;   88: udf     #0
;;   8c: udf     #0
;;   90: udf     #3
;;   94: udf     #0
;;   98: udf     #4
;;   9c: udf     #0
;;   a0: udf     #1
;;   a4: udf     #0
;;   a8: udf     #2
;;   ac: udf     #0
