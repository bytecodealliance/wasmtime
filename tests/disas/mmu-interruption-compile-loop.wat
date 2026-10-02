;;! target = "x86_64"
;;! test = "compile"
;;! flags = ["-Wmmu-interruption=y"]
;;! objdump = "--traps"

;; Nail down codegen for the snippet in mmu_interrupt_check_offsets() test. If
;; this starts failing, that may need the offsets in its assert reexamined.

(module
  (memory 0)
  (func (loop (br 0)))
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r8
;;       movq    0x10(%r8), %r11
;;       movq    (%r11), %r10
;;       ╰─╼ trap: MmuInterrupt
;;       movq    (%r11), %r10
;;       ╰─╼ trap: MmuInterrupt
;;       jmp     0xf
