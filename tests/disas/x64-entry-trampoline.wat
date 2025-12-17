;;! target = "x86_64"
;;! test = "compile"
;;! objdump = "--filter array_to_wasm --funcs all"

(module (func (export "")))

;; wasm[0]::array_to_wasm_trampoline[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       subq    $0x30, %rsp
;;       movq    %rbx, (%rsp)
;;       movq    %r12, 8(%rsp)
;;       movq    %r13, 0x10(%rsp)
;;       movq    %r14, 0x18(%rsp)
;;       movq    %r15, 0x20(%rsp)
;;       movq    8(%rdi), %rcx
;;       movq    %rbp, %rdx
;;       movq    %rdx, 0x48(%rcx)
;;       movq    %rsp, %rdx
;;       movq    %rdx, 0x40(%rcx)
;;       leaq    0x2f(%rip), %r8
;;       movq    %r8, 0x50(%rcx)
;;       callq   0
;;       ├─╼ exception frame offset: SP = FP - 0x30
;;       ╰─╼ exception handler: default handler, no dynamic context, handler=0x71
;;       movl    $1, %eax
;;       movq    (%rsp), %rbx
;;       movq    8(%rsp), %r12
;;       movq    0x10(%rsp), %r13
;;       movq    0x18(%rsp), %r14
;;       movq    0x20(%rsp), %r15
;;       addq    $0x30, %rsp
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;   71: xorl    %eax, %eax
;;   73: movq    (%rsp), %rbx
;;   77: movq    8(%rsp), %r12
;;   7c: movq    0x10(%rsp), %r13
;;   81: movq    0x18(%rsp), %r14
;;   86: movq    0x20(%rsp), %r15
;;   8b: addq    $0x30, %rsp
;;   8f: movq    %rbp, %rsp
;;   92: popq    %rbp
;;   93: retq
