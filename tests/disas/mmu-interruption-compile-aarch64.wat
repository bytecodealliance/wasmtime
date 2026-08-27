;;! target = "aarch64"
;;! test = "compile"
;;! flags = ["-Wmmu-interruption=y"]

(module
  (memory 0)
  (func)
)
;; wasm[0]::function[0]:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       ldr     x4, [x2, #8]
;;       ldr     x4, [x4, #0x10]
;;       mov     x0, x2
;;       ldr     x9, [x4]
;;       ldp     x29, x30, [sp], #0x10
;;       ret
