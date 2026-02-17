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
;;       ldr     x11, [x0, #8]
;;       mov     x12, x29
;;       str     x12, [x11, #0x48]
;;       mov     x12, sp
;;       str     x12, [x11, #0x40]
;;       adr     x13, #0x94
;;       str     x13, [x11, #0x50]
;;       mov     x2, x0
;;       mov     x3, x1
;;       bl      #0
;;       ├─╼ exception frame offset: SP = FP - 0x90
;;       ╰─╼ exception handler: default handler, no dynamic context, handler=0x94
;;   64: mov     w0, #1
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
;;   94: mov     w0, #0
;;   98: ldp     d8, d9, [sp], #0x10
;;   9c: ldp     d10, d11, [sp], #0x10
;;   a0: ldp     d12, d13, [sp], #0x10
;;   a4: ldp     d14, d15, [sp], #0x10
;;   a8: ldp     x19, x20, [sp], #0x10
;;   ac: ldp     x21, x22, [sp], #0x10
;;   b0: ldp     x23, x24, [sp], #0x10
;;   b4: ldp     x25, x26, [sp], #0x10
;;   b8: ldp     x27, x28, [sp], #0x10
;;   bc: ldp     x29, x30, [sp], #0x10
;;   c0: ret
