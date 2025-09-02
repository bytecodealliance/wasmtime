;;! target = "x86_64"
;;! test = "compile"
;;! objdump = "--filter array_to_wasm --funcs all"

(module (func (export "")))

;; wasm[0]::array_to_wasm_trampoline[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movl    (%rdi), %r9d
;;       cmpl    $0x65726f63, %r9d
;;       jne     0x37
;;   1d: movq    8(%rdi), %r11
;;       movq    %rbp, %rax
;;       movq    %rax, 0x38(%r11)
;;       callq   0
;;       movl    $1, %eax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;   37: ud2
