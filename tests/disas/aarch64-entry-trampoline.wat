;;! target = "aarch64"
;;! test = "compile"
;;! objdump = "--filter array_to_wasm --funcs all"

(module (func (export "")))

;; wasm[0]::array_to_wasm_trampoline[0]:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       ldr     w10, [x0]
;;       mov     w9, #0x6f63
;;       movk    w9, #0x6572, lsl #16
;;       cmp     w10, w9
;;       cset    x13, eq
;;       uxtb    w11, w13
;;       cbz     x11, #0x58
;;   34: ldr     x12, [x0, #8]
;;       mov     x13, x29
;;       str     x13, [x12, #0x38]
;;       mov     x2, x0
;;       mov     x3, x1
;;       bl      #0
;;   4c: mov     w0, #1
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;   58: .byte   0x1f, 0xc1, 0x00, 0x00
