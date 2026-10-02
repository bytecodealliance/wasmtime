;;! target = "x86_64"
;;! test = "compile"
;;! flags = ["-Wmmu-interruption=y"]
;;! objdump = "--traps"

(module
  (memory 0)
  (func)
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %rsi
;;       movq    0x10(%rsi), %r11
;;       movq    (%r11), %r10
;;       ╰─╼ trap: MmuInterrupt
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
