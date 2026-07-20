;;! target = "x86_64"
;;! test = "compile"
;;! flags = ["-Wmmu-interruption=y"]

(module
  (memory 0)
  (func)
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %rsi
;;       movq    0x10(%rsi), %rsi
;;       movq    (%rsi), %r10
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
