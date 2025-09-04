;;! target = "aarch64"
;;! test = "compile"
;;! objdump = "--filter array_to_wasm --funcs all"

(module (func (export "")))

;; wasm[0]::array_to_wasm_trampoline[0]:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       ldr     x5, [x0, #8]
;;       mov     x6, x29
;;       str     x6, [x5, #0x38]
;;       mov     x2, x0
;;       mov     x3, x1
;;       bl      #0
;;   30: mov     w0, #1
;;       ldp     x29, x30, [sp], #0x10
;;       ret
