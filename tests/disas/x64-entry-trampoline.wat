;;! target = "x86_64"
;;! test = "compile"
;;! objdump = "--filter array_to_wasm --funcs all"

(module (func (export "")))

;; wasm[0]::array_to_wasm_trampoline[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r8
;;       movq    %rbp, %r9
;;       movq    %r9, 0x38(%r8)
;;       callq   0
;;       movl    $1, %eax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
