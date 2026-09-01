;;! target = "aarch64"
;;! test = "winch"
;;! flags = "-W exceptions -C collector=null"

;; A `catch_ref` landing pad appends the exception reference after the tag's
;; payload fields before branching to its target.
(module
  (tag $e (param funcref))
  (func
    (block $h (result funcref exnref)
      (try_table (result funcref) (catch_ref $e $h)
        (throw $e (ref.null func)))
      (unreachable))
    (throw_ref)))
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
;;       b.lo    #0x2d0
;;   2c: mov     x9, x0
;;       sub     x28, x28, #0x10
;;       mov     sp, x28
;;       stur    x0, [x28, #8]
;;       stur    x1, [x28]
;;       mov     x0, x9
;;       bl      #0x51c
;;       ├─╼ exception frame offset: SP = FP - 0x20
;;       ╰─╼ exception handler: tag=0, context at [SP+0x8], handler=0x1dc
;;   48: ldur    x9, [x28, #8]
;;       ldur    x16, [x9, #0x20]
;;       ldur    w1, [x16]
;;       adds    w1, w1, #7
;;       b.hs    #0x2d4
;;   5c: and     w1, w1, #0xfffffff8
;;       mov     w2, w1
;;       adds    w2, w2, #0x18
;;       b.hs    #0x2d8
;;   6c: sub     x28, x28, #4
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
;;       b.ls    #0xdc
;;       b       #0xb4
;;   b4: sub     x0, x0, x2, uxtx
;;       sub     x28, x28, #8
;;       mov     sp, x28
;;       stur    x0, [x28]
;;       mov     x0, x9
;;       ldur    x1, [x28]
;;       bl      #0x440
;;       ├─╼ exception frame offset: SP = FP - 0x30
;;       ╰─╼ exception handler: tag=0, context at [SP+0x18], handler=0x1dc
;;   d0: add     x28, x28, #8
;;       mov     sp, x28
;;       ldur    x9, [x28, #0x10]
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
;;       ldur    w16, [x16, #0xc]
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
;;       mov     x1, #0
;;       sub     x28, x28, #8
;;       mov     sp, x28
;;       stur    x2, [x28]
;;       sub     x28, x28, #4
;;       mov     sp, x28
;;       stur    w0, [x28]
;;       sub     x28, x28, #8
;;       mov     sp, x28
;;       stur    x1, [x28]
;;       sub     x28, x28, #0xc
;;       mov     sp, x28
;;       mov     x0, x9
;;       ldur    x1, [x28, #0xc]
;;       bl      #0x494
;;       ├─╼ exception frame offset: SP = FP - 0x40
;;       ╰─╼ exception handler: tag=0, context at [SP+0x28], handler=0x1dc
;;  178: add     x28, x28, #0xc
;;       mov     sp, x28
;;       add     x28, x28, #8
;;       mov     sp, x28
;;       ldur    x9, [x28, #0x14]
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
;;       sub     x28, x28, #0xc
;;       mov     sp, x28
;;       mov     x0, x9
;;       ldur    w1, [x28, #0xc]
;;       bl      #0x54c
;;       ├─╼ exception frame offset: SP = FP - 0x30
;;       ╰─╼ exception handler: tag=0, context at [SP+0x18], handler=0x1dc
;;  1c8: add     x28, x28, #0xc
;;       mov     sp, x28
;;       add     x28, x28, #4
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
;;       add     x16, x16, #0x18
;;       cmp     x16, x2, uxtx
;;       b.hi    #0x2dc
;;  210: mov     x2, x1
;;       add     x2, x2, x0, uxtx
;;       ldur    w1, [x2, #0x10]
;;       sub     x28, x28, #8
;;       mov     sp, x28
;;       stur    x2, [x28]
;;       sub     x28, x28, #4
;;       mov     sp, x28
;;       stur    w0, [x28]
;;       sub     x28, x28, #4
;;       mov     sp, x28
;;       stur    w1, [x28]
;;       mov     x0, x9
;;       ldur    w1, [x28]
;;       mov     x2, #0xffffffff
;;       bl      #0x4c4
;;  250: add     x28, x28, #4
;;       ╰─╼ stack_map: frame_size=48, frame_offsets=[4]
;;       mov     sp, x28
;;       ldur    x9, [x28, #0x14]
;;       ldur    w1, [x28]
;;       add     x28, x28, #4
;;       mov     sp, x28
;;       ldur    x2, [x28]
;;       add     x28, x28, #8
;;       mov     sp, x28
;;       sub     x28, x28, #8
;;       mov     sp, x28
;;       stur    x0, [x28]
;;       mov     w0, w1
;;       sub     x28, x28, #4
;;       mov     sp, x28
;;       stur    w0, [x28]
;;       sub     x28, x28, #4
;;       mov     sp, x28
;;       mov     x0, x9
;;       ldur    w1, [x28, #4]
;;       bl      #0x54c
;;  2a4: add     x28, x28, #4
;;       ╰─╼ stack_map: frame_size=48, frame_offsets=[4]
;;       mov     sp, x28
;;       add     x28, x28, #4
;;       mov     sp, x28
;;       ldur    x9, [x28, #0x10]
;;       add     x28, x28, #0x10
;;       mov     sp, x28
;;       mov     sp, x28
;;       ldr     x28, [sp], #0x10
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;  2d0: udf     #0xc11f
;;  2d4: udf     #0xc11f
;;  2d8: udf     #0xc11f
;;  2dc: udf     #0xc11f
