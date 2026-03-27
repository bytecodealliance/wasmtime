;;! target = "x86_64"
;;! test = "compile"
;;! flags = ["-Wepoch-interruption-via-mmu=y"]

(module
  (memory 0)
  (func)
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %rdx
;;       movq    0x10(%rdx), %rdx
;;       movq    (%rdx), %rdx
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
