;;! target = "aarch64"
;;! test = "winch"
;;! flags = "-W exceptions -C collector=copying"

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
;;       b.lo    #0x264
;;   2c: mov     x9, x0
;;       sub     x28, x28, #0x10
;;       mov     sp, x28
;;       stur    x0, [x28, #8]
;;       stur    x1, [x28]
;;       mov     x0, x9
;;       bl      #0x4a4
;;       ├─╼ exception frame offset: SP = FP - 0x20
;;       ╰─╼ exception handler: tag=0, context at [SP+0x8], handler=0x170
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
;;       mov     w1, #2
;;       movk    w1, #0x400, lsl #16
;;       ldur    w2, [x28, #8]
;;       mov     x3, #0x20
;;       mov     x4, #0x10
;;       bl      #0x3cc
;;       ├─╼ exception frame offset: SP = FP - 0x30
;;       ╰─╼ exception handler: tag=0, context at [SP+0x18], handler=0x170
;;   90: add     x28, x28, #8
;;       mov     sp, x28
;;       add     x28, x28, #4
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
;;       stur    w1, [x2, #0x10]
;;       mov     x16, #0
;;       stur    w16, [x2, #0x14]
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
;;       bl      #0x41c
;;       ├─╼ exception frame offset: SP = FP - 0x40
;;       ╰─╼ exception handler: tag=0, context at [SP+0x28], handler=0x170
;;  10c: add     x28, x28, #0xc
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
;;       stur    w0, [x2, #0x18]
;;       sub     x28, x28, #4
;;       mov     sp, x28
;;       stur    w1, [x28]
;;       sub     x28, x28, #0xc
;;       mov     sp, x28
;;       mov     x0, x9
;;       ldur    w1, [x28, #0xc]
;;       bl      #0x4d4
;;       ├─╼ exception frame offset: SP = FP - 0x30
;;       ╰─╼ exception handler: tag=0, context at [SP+0x18], handler=0x170
;;  15c: add     x28, x28, #0xc
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
;;       add     x16, x16, #0x20
;;       cmp     x16, x2, uxtx
;;       b.hi    #0x268
;;  1a4: mov     x2, x1
;;       add     x2, x2, x0, uxtx
;;       ldur    w1, [x2, #0x18]
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
;;       bl      #0x44c
;;  1e4: add     x28, x28, #4
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
;;       bl      #0x4d4
;;  238: add     x28, x28, #4
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
;;  264: udf     #0xc11f
;;  268: udf     #0xc11f
