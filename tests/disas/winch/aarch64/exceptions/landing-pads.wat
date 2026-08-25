;;! target = "aarch64"
;;! test = "winch"
;;! flags = "-W exceptions -C collector=null"

;; A call with tagged and default handlers records both landing pads.
(module
  (tag $e (param i32))
  (import "m" "may_throw" (func $may_throw (param i32)))

  (func (param i32) (result i32)
    (block $catch_all
      (block $tagged (result i32)
        (try_table (catch $e $tagged) (catch_all $catch_all)
          (call $may_throw (local.get 0)))
        (return (i32.const 0)))
      (return))
    (i32.const -1)))
;; wasm[0]::function[1]:
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
;;       b.lo    #0x118
;;   2c: mov     x9, x0
;;       sub     x28, x28, #0x18
;;       mov     sp, x28
;;       stur    x0, [x28, #0x10]
;;       stur    x1, [x28, #8]
;;       stur    w2, [x28, #4]
;;       ldur    x4, [x9, #0x48]
;;       ldur    x3, [x9, #0x38]
;;       ldur    w16, [x28, #4]
;;       sub     x28, x28, #4
;;       mov     sp, x28
;;       stur    w16, [x28]
;;       sub     x28, x28, #4
;;       mov     sp, x28
;;       mov     x0, x4
;;       mov     x1, x9
;;       ldur    w2, [x28, #4]
;;       blr     x3
;;       ├─╼ exception frame offset: SP = FP - 0x30
;;       ├─╼ exception handler: tag=0, context at [SP+0x18], handler=0xa8
;;       ╰─╼ exception handler: default handler, context at [SP+0x18], handler=0x8c
;;   74: add     x28, x28, #4
;;       mov     sp, x28
;;       add     x28, x28, #4
;;       mov     sp, x28
;;       ldur    x9, [x28, #0x10]
;;       b       #0xf4
;;   8c: mov     x28, x29
;;       sub     x28, x28, #0x10
;;       mov     sp, x28
;;       sub     x28, x28, #0x18
;;       mov     sp, x28
;;       ldur    x9, [x28, #0x10]
;;       b       #0xfc
;;   a8: mov     x28, x29
;;       sub     x28, x28, #0x10
;;       mov     sp, x28
;;       sub     x28, x28, #0x18
;;       mov     sp, x28
;;       ldur    x9, [x28, #0x10]
;;       ldur    x1, [x9, #8]
;;       ldur    x2, [x1, #0x28]
;;       ldur    x1, [x1, #0x20]
;;       mov     x16, x0
;;       add     x16, x16, #0x18
;;       cmp     x16, x2, uxtx
;;       sub     sp, x28, #8
;;       b.hi    #0x11c
;;   e0: mov     sp, x28
;;       mov     x2, x1
;;       add     x2, x2, x0, uxtx
;;       ldur    w0, [x2, #0x10]
;;       b       #0x100
;;   f4: mov     x0, #0
;;       b       #0x100
;;   fc: mov     x0, #0xffffffff
;;       add     x28, x28, #0x18
;;       mov     sp, x28
;;       mov     sp, x28
;;       ldr     x28, [sp], #0x10
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;  118: udf     #0xc11f
;;  11c: udf     #0xc11f
