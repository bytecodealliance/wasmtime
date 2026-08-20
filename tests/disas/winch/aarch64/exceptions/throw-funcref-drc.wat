;;! target = "aarch64"
;;! test = "winch"
;;! flags = "-W exceptions -C collector=drc"

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
;;       b.lo    #0x1a0
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
;;       bl      #0x29c
;;   5c: ldur    x9, [x28, #0x18]
;;       ldur    x1, [x9, #0x28]
;;       ldur    w1, [x1, #4]
;;       sub     x28, x28, #4
;;       mov     sp, x28
;;       stur    w0, [x28]
;;       sub     x28, x28, #4
;;       mov     sp, x28
;;       stur    w1, [x28]
;;       sub     x28, x28, #8
;;       mov     sp, x28
;;       mov     x0, x9
;;       mov     x1, #0x4000000
;;       ldur    w2, [x28, #8]
;;       mov     x3, #0x28
;;       mov     x4, #8
;;       bl      #0x21c
;;   a0: add     x28, x28, #8
;;       mov     sp, x28
;;       add     x28, x28, #4
;;       mov     sp, x28
;;       ldur    x9, [x28, #0x1c]
;;       ldur    x1, [x9, #8]
;;       ldur    x2, [x1, #0x28]
;;       ldur    x1, [x1, #0x20]
;;       mov     x2, x1
;;       add     x2, x2, x0, uxtx
;;       ldur    w1, [x28]
;;       add     x28, x28, #4
;;       mov     sp, x28
;;       stur    w1, [x2, #0x18]
;;       mov     x16, #0
;;       stur    w16, [x2, #0x1c]
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
;;       bl      #0x26c
;;  124: add     x28, x28, #4
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
;;       stur    w0, [x2, #0x20]
;;       sub     x28, x28, #4
;;       mov     sp, x28
;;       stur    w1, [x28]
;;       sub     x28, x28, #4
;;       mov     sp, x28
;;       mov     x0, x9
;;       ldur    w1, [x28, #4]
;;       bl      #0x2cc
;;  174: add     x28, x28, #4
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
;;  1a0: udf     #0xc11f
