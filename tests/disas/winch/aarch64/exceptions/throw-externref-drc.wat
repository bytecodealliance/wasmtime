;;! target = "aarch64"
;;! test = "winch"
;;! flags = "-W exceptions -C collector=drc"

;; Store an external reference in an exception payload.
(module
  (tag $e (param externref))
  (func (param externref)
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
;;       b.lo    #0x1a4
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
;;       sub     x28, x28, #4
;;       mov     sp, x28
;;       mov     x0, x9
;;       bl      #0x274
;;   64: add     x28, x28, #4
;;       ╰─╼ stack_map: frame_size=48, frame_offsets=[4, 12]
;;       mov     sp, x28
;;       ldur    x9, [x28, #0x14]
;;       ldur    x1, [x9, #0x28]
;;       ldur    w1, [x1, #4]
;;       sub     x28, x28, #4
;;       mov     sp, x28
;;       stur    w0, [x28]
;;       sub     x28, x28, #4
;;       mov     sp, x28
;;       stur    w1, [x28]
;;       sub     x28, x28, #0xc
;;       mov     sp, x28
;;       mov     x0, x9
;;       mov     x1, #0x4000000
;;       ldur    w2, [x28, #0xc]
;;       mov     x3, #0x28
;;       mov     x4, #8
;;       bl      #0x224
;;   b0: add     x28, x28, #0xc
;;       ╰─╼ stack_map: frame_size=64, frame_offsets=[20, 28]
;;       mov     sp, x28
;;       add     x28, x28, #4
;;       mov     sp, x28
;;       ldur    x9, [x28, #0x18]
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
;;       ldur    w1, [x28]
;;       add     x28, x28, #4
;;       mov     sp, x28
;;       tst     w1, w1
;;       b.eq    #0x154
;;       b       #0x108
;;  108: mov     w16, w1
;;       and     w16, w16, #1
;;       tst     w16, w16
;;       b.ne    #0x154
;;       b       #0x11c
;;  11c: ldur    x3, [x9, #8]
;;       ldur    x4, [x3, #0x28]
;;       ldur    x3, [x3, #0x20]
;;       mov     x16, x1
;;       add     x16, x16, #0x10
;;       cmp     x16, x4, uxtx
;;       sub     sp, x28, #8
;;       b.hi    #0x1a8
;;  13c: mov     sp, x28
;;       mov     x5, x3
;;       add     x5, x5, x1, uxtx
;;       ldur    x6, [x5, #8]
;;       add     x6, x6, #1
;;       stur    x6, [x5, #8]
;;       stur    w1, [x2, #0x20]
;;       sub     x28, x28, #4
;;       mov     sp, x28
;;       stur    w0, [x28]
;;       sub     x28, x28, #4
;;       mov     sp, x28
;;       mov     x0, x9
;;       ldur    w1, [x28, #4]
;;       bl      #0x2a4
;;  178: add     x28, x28, #4
;;       ╰─╼ stack_map: frame_size=48, frame_offsets=[12]
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
;;  1a4: udf     #0xc11f
;;  1a8: udf     #0xc11f
