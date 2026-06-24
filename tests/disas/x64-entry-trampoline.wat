;;! target = "x86_64"
;;! test = "compile"
;;! objdump = "--filter array_to_wasm --funcs all"

(module (func (export "")))

;; wasm[0]::array_to_wasm_trampoline[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       subq    $0x40, %rsp
;;       movq    %rbx, 0x10(%rsp)
;;       movq    %r12, 0x18(%rsp)
;;       movq    %r13, 0x20(%rsp)
;;       movq    %r14, 0x28(%rsp)
;;       movq    %r15, 0x30(%rsp)
;;       movq    %rbp, %rcx
;;       movq    8(%rdi), %r8
;;       movq    %rcx, 0x48(%r8)
;;       movq    %rsp, %rcx
;;       movq    %rcx, 0x40(%r8)
;;       leaq    0x34(%rip), %rcx
;;       movq    %rcx, 0x50(%r8)
;;       movq    %r8, (%rsp)
;;       callq   0
;;       ├─╼ exception frame offset: SP = FP - 0x40
;;       ╰─╼ exception handler: default handler, no dynamic context, handler=0x77
;;       movl    $1, %eax
;;       movq    0x10(%rsp), %rbx
;;       movq    0x18(%rsp), %r12
;;       movq    0x20(%rsp), %r13
;;       movq    0x28(%rsp), %r14
;;       movq    0x30(%rsp), %r15
;;       addq    $0x40, %rsp
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;   77: movq    (%rsp), %r8
;;   7b: movq    $1, 0x88(%r8)
;;   86: xorl    %eax, %eax
;;   88: movq    0x10(%rsp), %rbx
;;   8d: movq    0x18(%rsp), %r12
;;   92: movq    0x20(%rsp), %r13
;;   97: movq    0x28(%rsp), %r14
;;   9c: movq    0x30(%rsp), %r15
;;   a1: addq    $0x40, %rsp
;;   a5: movq    %rbp, %rsp
;;   a8: popq    %rbp
;;   a9: retq
