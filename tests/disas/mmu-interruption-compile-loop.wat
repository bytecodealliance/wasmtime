;;! target = "x86_64"
;;! test = "compile"
;;! flags = ["-Wmmu-interruption=y"]

;; Nail down codegen for the snippet in mmu_interrupt_check_offsets() test. If
;; this starts failing, that may need the offsets in its assert reexamined.

(module
  (memory 0)
  (func (loop (br 0)))
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %rsi
;;       movq    0x10(%rsi), %rsi
;;       movq    (%rsi), %r10
;;       movq    (%rsi), %r10
;;       jmp     0xf
