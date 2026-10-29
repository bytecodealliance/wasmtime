;;! target = "aarch64"
;;! test = "compile"
;;! objdump = "--filter array_to_wasm --funcs all"

(module (func (export "")))

;; wasm[0]::array_to_wasm_trampoline[0]:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       stp     x27, x28, [sp, #-0x10]!
;;       stp     x25, x26, [sp, #-0x10]!
;;       stp     x23, x24, [sp, #-0x10]!
;;       stp     x21, x22, [sp, #-0x10]!
;;       stp     x19, x20, [sp, #-0x10]!
;;       stp     d14, d15, [sp, #-0x10]!
;;       stp     d12, d13, [sp, #-0x10]!
;;       stp     d10, d11, [sp, #-0x10]!
;;       stp     d8, d9, [sp, #-0x10]!
;;       sub     sp, sp, #0x10
;;       mov     x3, x0
;;       mov     x13, x29
;;       mov     x0, x3
;;       ldr     x2, [x0, #8]
;;       str     x13, [x2, #0x48]
;;       mov     x14, sp
;;       str     x14, [x2, #0x40]
;;       adr     x15, #0xa8
;;       str     x15, [x2, #0x50]
;;       stur    x2, [sp]
;;       mov     x2, x3
;;       mov     x3, x1
;;       bl      #0
;;       ├─╼ exception frame offset: SP = FP - 0xa0
;;       ╰─╼ exception handler: default handler, no dynamic context, handler=0xa8
;;   74: mov     w0, #1
;;       add     sp, sp, #0x10
;;       ldp     d8, d9, [sp], #0x10
;;       ldp     d10, d11, [sp], #0x10
;;       ldp     d12, d13, [sp], #0x10
;;       ldp     d14, d15, [sp], #0x10
;;       ldp     x19, x20, [sp], #0x10
;;       ldp     x21, x22, [sp], #0x10
;;       ldp     x23, x24, [sp], #0x10
;;       ldp     x25, x26, [sp], #0x10
;;       ldp     x27, x28, [sp], #0x10
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;   a8: mov     x1, #1
;;   ac: ldur    x2, [sp]
;;   b0: str     x1, [x2, #0x88]
;;   b4: mov     x1, #0
;;   b8: str     x1, [x2, #0xa0]
;;   bc: mov     w0, #0
;;   c0: add     sp, sp, #0x10
;;   c4: ldp     d8, d9, [sp], #0x10
;;   c8: ldp     d10, d11, [sp], #0x10
;;   cc: ldp     d12, d13, [sp], #0x10
;;   d0: ldp     d14, d15, [sp], #0x10
;;   d4: ldp     x19, x20, [sp], #0x10
;;   d8: ldp     x21, x22, [sp], #0x10
;;   dc: ldp     x23, x24, [sp], #0x10
;;   e0: ldp     x25, x26, [sp], #0x10
;;   e4: ldp     x27, x28, [sp], #0x10
;;   e8: ldp     x29, x30, [sp], #0x10
;;   ec: ret
