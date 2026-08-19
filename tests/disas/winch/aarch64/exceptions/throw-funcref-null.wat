;;! target = "aarch64"
;;! test = "winch"
;;! flags = "-W exceptions -C collector=null"

;; A function reference is interned before it is stored in the exception.
(module
  (tag $e (param funcref))
  (func (param funcref)
    (throw $e (local.get 0))))
;; wasm[0]::function[0]:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       str     x28, [sp, #-0x10]!
;;       mov     x28, sp
;;       ldur    x16, [x0, #8]
;;       ldur    x16, [x16, #0x18]
;;       mov     x17, #0
;;       movk    x17, #0x30
;;       add     x16, x16, x17
;;       cmp     sp, x16
;;       b.lo    #0x210
;;   2c: mov     x9, x0
;;       sub     x28, x28, #0x18
;;       mov     sp, x28
;;       stur    x0, [x28, #0x10]
;;       stur    x1, [x28, #8]
;;       stur    x2, [x28]
;;       ldur    x16, [x28]
;;       sub     x28, x28, #8
;;       mov     sp, x28
;;       stur    x16, [x28]
;;       mov     x0, x9
;;       bl      #0x318
;;   5c: ldur    x9, [x28, #0x18]
;;       ldur    x16, [x9, #0x20]
;;       ldur    w1, [x16]
;;       adds    w1, w1, #7
;;       b.hs    #0x214
;;   70: and     w1, w1, #0xfffffff8
;;       mov     w2, w1
;;       adds    w2, w2, #0x18
;;       b.hs    #0x218
;;   80: sub     x28, x28, #4
;;       mov     sp, x28
;;       stur    w0, [x28]
;;       sub     x28, x28, #4
;;       mov     sp, x28
;;       stur    w1, [x28]
;;       sub     x28, x28, #4
;;       mov     sp, x28
;;       stur    w2, [x28]
;;       ldur    w0, [x28]
;;       add     x28, x28, #4
;;       mov     sp, x28
;;       ldur    x1, [x9, #8]
;;       ldur    x2, [x1, #0x28]
;;       ldur    x1, [x1, #0x20]
;;       cmp     x0, x2, uxtx
;;       b.ls    #0xf0
;;       b       #0xc8
;;   c8: sub     x0, x0, x2, uxtx
;;       sub     x28, x28, #8
;;       mov     sp, x28
;;       stur    x0, [x28]
;;       mov     x0, x9
;;       ldur    x1, [x28]
;;       bl      #0x294
;;   e4: add     x28, x28, #8
;;       mov     sp, x28
;;       ldur    x9, [x28, #0x20]
;;       ldur    w0, [x28]
;;       add     x28, x28, #4
;;       mov     sp, x28
;;       ldur    x1, [x9, #8]
;;       ldur    x2, [x1, #0x28]
;;       ldur    x1, [x1, #0x20]
;;       mov     x2, x1
;;       add     x2, x2, x0, uxtx
;;       mov     w16, #0x18
;;       movk    w16, #0x400, lsl #16
;;       stur    w16, [x2]
;;       ldur    x16, [x9, #0x28]
;;       ldur    w16, [x16, #4]
;;       stur    w16, [x2, #4]
;;       ldur    x1, [x9, #0x20]
;;       mov     w16, w0
;;       add     w16, w16, #0x18
;;       stur    w16, [x1]
;;       ldur    w1, [x28]
;;       add     x28, x28, #4
;;       mov     sp, x28
;;       stur    w1, [x2, #8]
;;       mov     x16, #0
;;       stur    w16, [x2, #0xc]
;;       ldur    x1, [x28]
;;       add     x28, x28, #8
;;       mov     sp, x28
;;       sub     x28, x28, #8
;;       mov     sp, x28
;;       stur    x2, [x28]
;;       sub     x28, x28, #4
;;       mov     sp, x28
;;       stur    w0, [x28]
;;       sub     x28, x28, #8
;;       mov     sp, x28
;;       stur    x1, [x28]
;;       sub     x28, x28, #4
;;       mov     sp, x28
;;       mov     x0, x9
;;       ldur    x1, [x28, #4]
;;       bl      #0x2e8
;;  194: add     x28, x28, #4
;;       mov     sp, x28
;;       add     x28, x28, #8
;;       mov     sp, x28
;;       ldur    x9, [x28, #0x1c]
;;       ldur    w1, [x28]
;;       add     x28, x28, #4
;;       mov     sp, x28
;;       ldur    x2, [x28]
;;       add     x28, x28, #8
;;       mov     sp, x28
;;       stur    w0, [x2, #0x10]
;;       sub     x28, x28, #4
;;       mov     sp, x28
;;       stur    w1, [x28]
;;       sub     x28, x28, #4
;;       mov     sp, x28
;;       mov     x0, x9
;;       ldur    w1, [x28, #4]
;;       bl      #0x348
;;  1e4: add     x28, x28, #4
;;       mov     sp, x28
;;       add     x28, x28, #4
;;       mov     sp, x28
;;       ldur    x9, [x28, #0x10]
;;       add     x28, x28, #0x18
;;       mov     sp, x28
;;       mov     sp, x28
;;       ldr     x28, [sp], #0x10
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;  210: udf     #0xc11f
;;  214: udf     #0xc11f
;;  218: udf     #0xc11f
