;;! target = "aarch64"
;;! test = "winch"

(module
  (memory 1)
  (func (export "f") (param i32)
    (v128.store (i32.const 0)
      (select (v128.const i64x2 1 1) (v128.const i64x2 2 2) (local.get 0)))))
;; wasm[0]::function[0]:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       str     x28, [sp, #-0x10]!
;;       mov     x28, sp
;;       ldur    x16, [x0, #8]
;;       ldur    x16, [x16, #0x18]
;;       mov     x17, #0
;;       movk    x17, #0x18
;;       add     x16, x16, x17
;;       cmp     sp, x16
;;       b.lo    #0x94
;;   2c: mov     x9, x0
;;       sub     x28, x28, #0x18
;;       mov     sp, x28
;;       stur    x0, [x28, #0x10]
;;       stur    x1, [x28, #8]
;;       stur    w2, [x28, #4]
;;       ldur    w0, [x28, #4]
;;       ldr     q0, #0xa0
;;       ldr     q1, #0xb0
;;       cmp     w0, #0
;;       b.ne    #0x60
;;   58: mov     v0.16b, v0.16b
;;       b       #0x64
;;   60: mov     v0.16b, v1.16b
;;       mov     x0, #0
;;       ldur    x1, [x9, #0x38]
;;       add     x1, x1, w0, uxtw
;;       sub     sp, x28, #8
;;       stur    q0, [x1]
;;       mov     sp, x28
;;       add     x28, x28, #0x18
;;       mov     sp, x28
;;       mov     sp, x28
;;       ldr     x28, [sp], #0x10
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;   94: udf     #0xc11f
;;   98: udf     #0
;;   9c: udf     #0
;;   a0: udf     #2
;;   a4: udf     #0
;;   a8: udf     #2
;;   ac: udf     #0
;;   b0: udf     #1
;;   b4: udf     #0
;;   b8: udf     #1
;;   bc: udf     #0
