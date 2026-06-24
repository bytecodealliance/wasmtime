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
;;       mov     x3, x1
;;       mov     x12, x29
;;       ldr     x15, [x0, #8]
;;       str     x12, [x15, #0x48]
;;       mov     x13, sp
;;       str     x13, [x15, #0x40]
;;       adr     x14, #0xa0
;;       str     x14, [x15, #0x50]
;;       mov     x2, x0
;;       stur    x15, [sp]
;;       bl      #0
;;       ├─╼ exception frame offset: SP = FP - 0xa0
;;       ╰─╼ exception handler: default handler, no dynamic context, handler=0xa0
;;   6c: mov     w0, #1
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
;;   a0: mov     x0, #1
;;   a4: ldur    x15, [sp]
;;   a8: str     x0, [x15, #0x88]
;;   ac: mov     w0, #0
;;   b0: add     sp, sp, #0x10
;;   b4: ldp     d8, d9, [sp], #0x10
;;   b8: ldp     d10, d11, [sp], #0x10
;;   bc: ldp     d12, d13, [sp], #0x10
;;   c0: ldp     d14, d15, [sp], #0x10
;;   c4: ldp     x19, x20, [sp], #0x10
;;   c8: ldp     x21, x22, [sp], #0x10
;;   cc: ldp     x23, x24, [sp], #0x10
;;   d0: ldp     x25, x26, [sp], #0x10
;;   d4: ldp     x27, x28, [sp], #0x10
;;   d8: ldp     x29, x30, [sp], #0x10
;;   dc: ret
