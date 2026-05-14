;;! target = "x86_64"
;;! test = 'compile'
;;! flags = '-Ccranelift-wasmtime_debug_checks=on -Cmetadata-for-internal-asserts'
;;! objdump = '--funcs all'

(module (func))

;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; signatures[0]::wasm_to_array_trampoline:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       subq    $0x10, %rsp
;;       movq    %rbx, (%rsp)
;;       movl    (%rsi), %eax
;;       cmpl    $0x65726f63, %eax
;;       jne     0x70
;;   22: movq    8(%rsi), %rax
;;       movq    %rbp, %rcx
;;       movq    %rcx, 0x30(%rax)
;;       movq    %rbp, %rcx
;;       movq    8(%rcx), %rcx
;;       movq    %rcx, 0x38(%rax)
;;       movq    8(%rdi), %r8
;;       leaq    (%rsp), %rdx
;;       xorq    %rcx, %rcx
;;       movq    %rsi, %rbx
;;       callq   *%r8
;;       testb   %al, %al
;;       je      0x5e
;;   51: movq    (%rsp), %rbx
;;       addq    $0x10, %rsp
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;   5e: movq    0x10(%rbx), %rax
;;   62: movq    0x190(%rax), %rax
;;   69: movq    %rbx, %rdi
;;   6c: callq   *%rax
;;   6e: ud2
;;       ╰─╼ trap: InternalAssert
;;   70: ud2
;;       ╰─╼ trap: InternalAssert
