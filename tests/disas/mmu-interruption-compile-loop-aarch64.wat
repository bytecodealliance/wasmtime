;;! target = "aarch64"
;;! test = "compile"
;;! flags = ["-Wmmu-interruption=y"]
;;! objdump = "--traps"

(module
  (memory 0)
  (func (loop (br 0)))
)
;; wasm[0]::function[0]:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       ldr     x5, [x2, #8]
;;       ldr     x5, [x5, #0x10]
;;       mov     x0, x2
;;       ldr     x9, [x5]
;;       ╰─╼ trap: MmuInterrupt
;;       ldr     x9, [x5]
;;       ╰─╼ trap: MmuInterrupt
;;       b       #0x18
