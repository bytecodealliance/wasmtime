;;! target = "x86_64"
;;! test = "compile"
;;! flags = ["-Wepoch-interruption-via-mmu=y"]

;; Nail down codegen for the snippet in epoch_check_offsets() test. If this
;; starts failing, that may need the offsets in its assert reexamined.

(module
  (memory 0)
  (func (loop (br 0)))
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r8
;;       movq    0x10(%r8), %r8
;;       movq    (%r8), %r9
;;       movq    (%r8), %r10
;;       jmp     0xf
