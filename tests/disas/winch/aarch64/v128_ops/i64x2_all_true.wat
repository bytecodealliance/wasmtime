;;! target = "aarch64"
;;! test = "winch"

(module
  (func (export "run") (result i32)
    (i64x2.all_true (v128.const i64x2 1 0))))
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
;;       b.lo    #0x6c
;;   2c: mov     x9, x0
;;       sub     x28, x28, #0x10
;;       mov     sp, x28
;;       stur    x0, [x28, #8]
;;       stur    x1, [x28]
;;       ldr     q0, #0x70
;;       cmeq    v31.2d, v0.2d, #0
;;       addp    v31.2d, v31.2d, v31.2d
;;       fcmp    d31, d31
;;       cset    x0, eq
;;       add     x28, x28, #0x10
;;       mov     sp, x28
;;       mov     sp, x28
;;       ldr     x28, [sp], #0x10
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;   6c: udf     #0xc11f
;;   70: udf     #1
;;   74: udf     #0
;;   78: udf     #0
;;   7c: udf     #0
