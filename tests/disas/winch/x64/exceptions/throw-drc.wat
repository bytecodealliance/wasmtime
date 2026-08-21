;;! target = "x86_64"
;;! test = "winch"
;;! flags = "-W exceptions -C collector=drc"

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
;;       ja      0x12a
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movq    %r14, %rdi
;;       callq   0x231
;;       ├─╼ exception frame offset: SP = FP - 0x10
;;       ╰─╼ exception handler: tag=0, context at [SP+0x8], handler=0xea
;;       movq    8(%rsp), %r14
;;       movq    0x28(%r14), %rcx
;;       movl    8(%rcx), %ecx
;;       subq    $4, %rsp
;;       movl    %eax, (%rsp)
;;       subq    $4, %rsp
;;       movl    %ecx, (%rsp)
;;       subq    $8, %rsp
;;       movq    %r14, %rdi
;;       movl    $0x4000000, %esi
;;       movl    8(%rsp), %edx
;;       movl    $0x28, %ecx
;;       movl    $8, %r8d
;;       callq   0x1e2
;;       ├─╼ exception frame offset: SP = FP - 0x20
;;       ╰─╼ exception handler: tag=0, context at [SP+0x18], handler=0xea
;;       addq    $8, %rsp
;;       addq    $4, %rsp
;;       movq    0xc(%rsp), %r14
;;       movq    8(%r14), %rcx
;;       movq    0x28(%rcx), %rdx
;;       movq    0x20(%rcx), %rcx
;;       movq    %rcx, %rdx
;;       addq    %rax, %rdx
;;       movl    (%rsp), %ecx
;;       addq    $4, %rsp
;;       movl    %ecx, 0x18(%rdx)
;;       movl    $0, 0x1c(%rdx)
;;       movl    $0x2a, 0x20(%rdx)
;;       subq    $4, %rsp
;;       movl    %eax, (%rsp)
;;       subq    $0xc, %rsp
;;       movq    %r14, %rdi
;;       movl    0xc(%rsp), %esi
;;       callq   0x25e
;;       ├─╼ exception frame offset: SP = FP - 0x20
;;       ╰─╼ exception handler: tag=0, context at [SP+0x18], handler=0xea
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
;;       addq    $0x28, %r11
;;       cmpq    %rdx, %r11
;;       ja      0x12c
;;  118: movq    %rcx, %rdx
;;       addq    %rax, %rdx
;;       movl    0x20(%rdx), %eax
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;  12a: ud2
;;  12c: ud2
