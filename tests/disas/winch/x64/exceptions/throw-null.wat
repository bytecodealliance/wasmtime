;;! target = "x86_64"
;;! test = "winch"
;;! flags = "-W exceptions -C collector=null"

;; Calls made while the `try_table` handler is active carry exception metadata.
;; Its landing pad loads the exception's payload and branches to `$h`.
(module
  (tag $e (param i32))
  (func (result i32)
    (block $h (result i32)
      (try_table (result i32) (catch $e $h)
        (throw $e (i32.const 42))))))
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x18(%r11), %r11
;;       addq    $0x20, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x188
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movq    %r14, %rdi
;;       callq   0x28b
;;       ├─╼ exception frame offset: SP = FP - 0x10
;;       ╰─╼ exception handler: tag=0, context at [SP+0x8], handler=0x146
;;       movq    8(%rsp), %r14
;;       movq    0x20(%r14), %r11
;;       movl    (%r11), %ecx
;;       addl    $7, %ecx
;;       jb      0x18a
;;   4f: andl    $0xfffffff8, %ecx
;;       movl    %ecx, %edx
;;       addl    $0x18, %edx
;;       jb      0x18c
;;   63: subq    $4, %rsp
;;       movl    %eax, (%rsp)
;;       subq    $4, %rsp
;;       movl    %ecx, (%rsp)
;;       subq    $4, %rsp
;;       movl    %edx, (%rsp)
;;       movl    (%rsp), %eax
;;       addq    $4, %rsp
;;       movq    8(%r14), %rcx
;;       movq    0x28(%rcx), %rdx
;;       movq    0x20(%rcx), %rcx
;;       cmpq    %rdx, %rax
;;       jbe     0xbc
;;   a0: subq    %rdx, %rax
;;       pushq   %rax
;;       movq    %r14, %rdi
;;       movq    (%rsp), %rsi
;;       callq   0x244
;;       ├─╼ exception frame offset: SP = FP - 0x20
;;       ╰─╼ exception handler: tag=0, context at [SP+0x18], handler=0x146
;;       addq    $8, %rsp
;;       movq    0x10(%rsp), %r14
;;       movl    (%rsp), %eax
;;       addq    $4, %rsp
;;       movq    8(%r14), %rcx
;;       movq    0x28(%rcx), %rdx
;;       movq    0x20(%rcx), %rcx
;;       movq    %rcx, %rdx
;;       addq    %rax, %rdx
;;       movl    $0x4000018, (%rdx)
;;       movq    0x28(%r14), %r11
;;       movl    8(%r11), %r11d
;;       movl    %r11d, 4(%rdx)
;;       movq    0x20(%r14), %rcx
;;       movl    %eax, %r11d
;;       addl    $0x18, %r11d
;;       movl    %r11d, (%rcx)
;;       movl    (%rsp), %ecx
;;       addq    $4, %rsp
;;       movl    %ecx, 8(%rdx)
;;       movl    $0, 0xc(%rdx)
;;       movl    $0x2a, 0x10(%rdx)
;;       subq    $4, %rsp
;;       movl    %eax, (%rsp)
;;       subq    $0xc, %rsp
;;       movq    %r14, %rdi
;;       movl    0xc(%rsp), %esi
;;       callq   0x2b8
;;       ├─╼ exception frame offset: SP = FP - 0x20
;;       ╰─╼ exception handler: tag=0, context at [SP+0x18], handler=0x146
;;       addq    $0xc, %rsp
;;       addq    $4, %rsp
;;       movq    8(%rsp), %r14
;;       movq    %rbp, %rsp
;;       subq    $0x10, %rsp
;;       movq    8(%rsp), %r14
;;       movq    8(%r14), %rcx
;;       movq    0x28(%rcx), %rdx
;;       movq    0x20(%rcx), %rcx
;;       movq    %rax, %r11
;;       addq    $0x18, %r11
;;       cmpq    %rdx, %r11
;;       ja      0x18e
;;  174: movq    %rcx, %rdx
;;       addq    %rax, %rdx
;;       movl    0x10(%rdx), %ecx
;;       movl    %ecx, %eax
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;  188: ud2
;;  18a: ud2
;;  18c: ud2
;;  18e: ud2
