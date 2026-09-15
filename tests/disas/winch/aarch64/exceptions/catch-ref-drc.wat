;;! target = "aarch64"
;;! test = "winch"
;;! flags = "-W exceptions -C collector=drc"

;; A `catch_ref` landing pad preserves the exception reference while the DRC
;; read barrier loads an `externref` payload.
(module
  (tag $e (param externref))
  (func
    (block $h (result externref exnref)
      (try_table (catch_ref $e $h)
        (throw $e (ref.null extern)))
      (unreachable))
    (drop)
    (drop)))
;; wasm[0]::function[0]:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       str     x28, [sp, #-0x10]!
;;       mov     x28, sp
;;       ldur    x16, [x0, #8]
;;       ldur    x16, [x16, #0x18]
;;       mov     x17, #0
;;       movk    x17, #0x20
;;       add     x16, x16, x17
;;       cmp     sp, x16
;;       b.lo    #0x2bc
;;   2c: mov     x9, x0
;;       sub     x28, x28, #0x10
;;       mov     sp, x28
;;       stur    x0, [x28, #8]
;;       stur    x1, [x28]
;;       mov     x0, x9
;;       bl      #0x47c
;;       ├─╼ exception frame offset: SP = FP - 0x20
;;       ╰─╼ exception handler: tag=0, context at [SP+0x8], handler=0x148
;;   48: ldur    x9, [x28, #8]
;;       ldur    x1, [x9, #0x28]
;;       ldur    w1, [x1, #0xc]
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
;;       bl      #0x42c
;;       ├─╼ exception frame offset: SP = FP - 0x30
;;       ╰─╼ exception handler: tag=0, context at [SP+0x18], handler=0x148
;;   8c: add     x28, x28, #0xc
;;       mov     sp, x28
;;       ldur    x9, [x28, #0xc]
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
;;       mov     x1, #0
;;       tst     w1, w1
;;       b.eq    #0x118
;;       b       #0xd4
;;   d4: mov     w16, w1
;;       and     w16, w16, #1
;;       tst     w16, w16
;;       b.ne    #0x118
;;       b       #0xe8
;;   e8: ldur    x3, [x9, #8]
;;       ldur    x4, [x3, #0x28]
;;       ldur    x3, [x3, #0x20]
;;       mov     x16, x1
;;       add     x16, x16, #0x10
;;       cmp     x16, x4, uxtx
;;       b.hi    #0x2c0
;;  104: mov     x5, x3
;;       add     x5, x5, x1, uxtx
;;       ldur    x6, [x5, #8]
;;       add     x6, x6, #1
;;       stur    x6, [x5, #8]
;;       stur    w1, [x2, #0x20]
;;       sub     x28, x28, #4
;;       mov     sp, x28
;;       stur    w0, [x28]
;;       sub     x28, x28, #0xc
;;       mov     sp, x28
;;       mov     x0, x9
;;       ldur    w1, [x28, #0xc]
;;       bl      #0x4ac
;;       ├─╼ exception frame offset: SP = FP - 0x30
;;       ╰─╼ exception handler: tag=0, context at [SP+0x18], handler=0x148
;;  13c: add     x28, x28, #0x10
;;       mov     sp, x28
;;       ldur    x9, [x28, #8]
;;       mov     x28, x29
;;       sub     x28, x28, #0x10
;;       mov     sp, x28
;;       sub     x28, x28, #0x10
;;       mov     sp, x28
;;       ldur    x9, [x28, #8]
;;       ldur    x1, [x9, #8]
;;       ldur    x2, [x1, #0x28]
;;       ldur    x1, [x1, #0x20]
;;       mov     x16, x0
;;       add     x16, x16, #0x28
;;       cmp     x16, x2, uxtx
;;       b.hi    #0x2c4
;;  17c: mov     x2, x1
;;       add     x2, x2, x0, uxtx
;;       ldur    w1, [x2, #0x20]
;;       sub     x28, x28, #8
;;       mov     sp, x28
;;       stur    x2, [x28]
;;       sub     x28, x28, #4
;;       mov     sp, x28
;;       stur    w0, [x28]
;;       sub     x28, x28, #4
;;       mov     sp, x28
;;       stur    w1, [x28]
;;       ldur    w0, [x28]
;;       tst     w0, w0
;;       b.eq    #0x268
;;       b       #0x1bc
;;  1bc: mov     w16, w0
;;       and     w16, w16, #1
;;       tst     w16, w16
;;       b.ne    #0x268
;;       b       #0x1d0
;;  1d0: ldur    x1, [x9, #8]
;;       ldur    x2, [x1, #0x28]
;;       ldur    x1, [x1, #0x20]
;;       mov     x16, x0
;;       add     x16, x16, #0x14
;;       cmp     x16, x2, uxtx
;;       b.hi    #0x2c8
;;  1ec: mov     x2, x1
;;       add     x2, x2, x0, uxtx
;;       ldur    w16, [x2]
;;       and     w16, w16, #2
;;       tst     w16, w16
;;       b.ne    #0x268
;;       b       #0x208
;;  208: ldur    x3, [x9, #0x20]
;;       ldur    w16, [x3]
;;       stur    w16, [x2, #0x10]
;;       ldur    w16, [x2]
;;       orr     w16, w16, #2
;;       stur    w16, [x2]
;;       ldur    x4, [x2, #8]
;;       add     x4, x4, #1
;;       stur    x4, [x2, #8]
;;       stur    w0, [x3]
;;       ldur    w4, [x3, #4]
;;       add     w4, w4, #1
;;       stur    w4, [x3, #4]
;;       ldur    w16, [x3, #8]
;;       add     w16, w16, w16, uxtx
;;       cmp     w4, w16, uxtx
;;       b.lo    #0x268
;;       b       #0x250
;;  250: cmp     w4, #0x400
;;       b.lo    #0x268
;;       b       #0x25c
;;  25c: mov     x0, x9
;;       bl      #0x500
;;  264: ldur    x9, [x28, #0x18]
;;       ╰─╼ stack_map: frame_size=48, frame_offsets=[0, 4]
;;       ldur    w0, [x28]
;;       add     x28, x28, #4
;;       mov     sp, x28
;;       ldur    w1, [x28]
;;       add     x28, x28, #4
;;       mov     sp, x28
;;       ldur    x2, [x28]
;;       add     x28, x28, #8
;;       mov     sp, x28
;;       sub     x28, x28, #4
;;       mov     sp, x28
;;       stur    w0, [x28]
;;       mov     w0, w1
;;       add     x28, x28, #4
;;       mov     sp, x28
;;       add     x28, x28, #0x10
;;       mov     sp, x28
;;       mov     sp, x28
;;       ldr     x28, [sp], #0x10
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;  2bc: udf     #0xc11f
;;  2c0: udf     #0xc11f
;;  2c4: udf     #0xc11f
;;  2c8: udf     #0xc11f
