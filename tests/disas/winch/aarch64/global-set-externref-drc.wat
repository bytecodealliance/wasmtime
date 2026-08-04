;;! target = "aarch64"
;;! test = "winch"
;;! flags = "-Ccollector=drc"

(module
  (global $g (mut externref) (ref.null extern))
  (func (param externref)
    (global.set $g (local.get 0))))
;; wasm[0]::function[0]:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       str     x28, [sp, #-0x10]!
;;       mov     x28, sp
;;       ldur    x16, [x0, #8]
;;       ldur    x16, [x16, #0x18]
;;       mov     x17, #0
;;       movk    x17, #0x1c
;;       add     x16, x16, x17
;;       cmp     sp, x16
;;       b.lo    #0xc8
;;   2c: mov     x9, x0
;;       sub     x28, x28, #0x18
;;       mov     sp, x28
;;       stur    x0, [x28, #0x10]
;;       stur    x1, [x28, #8]
;;       stur    w2, [x28, #4]
;;       ldur    w16, [x28, #4]
;;       sub     x28, x28, #4
;;       mov     sp, x28
;;       stur    w16, [x28]
;;       ldur    w0, [x28]
;;       add     x28, x28, #4
;;       mov     sp, x28
;;       ldur    x1, [x9, #8]
;;       ldur    x1, [x1, #0x20]
;;       ldur    w2, [x9, #0x30]
;;       tst     w0, w0
;;       b.eq    #0x8c
;;       b       #0x78
;;   78: mov     x3, x1
;;       add     x3, x3, x0, uxtx
;;       ldur    x4, [x3, #8]
;;       add     x4, x4, #1
;;       stur    x4, [x3, #8]
;;       stur    w0, [x9, #0x30]
;;       tst     w2, w2
;;       b.eq    #0xb0
;;       b       #0x9c
;;   9c: mov     x3, x1
;;       add     x3, x3, x2, uxtx
;;       ldur    x4, [x3, #8]
;;       sub     x4, x4, #1
;;       stur    x4, [x3, #8]
;;       add     x28, x28, #0x18
;;       mov     sp, x28
;;       mov     sp, x28
;;       ldr     x28, [sp], #0x10
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;   c8: udf     #0xc11f
