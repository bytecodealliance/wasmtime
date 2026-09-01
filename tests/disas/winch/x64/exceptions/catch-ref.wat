;;! target = "x86_64"
;;! test = "winch"
;;! flags = "-W exceptions -C collector=copying"

;; A `catch_ref` landing pad appends the exception reference after the tag's
;; payload fields before branching to its target.
(module
  (tag $e (param funcref))
  (func
    (block $h (result funcref exnref)
      (try_table (result funcref) (catch_ref $e $h)
        (throw $e (ref.null func)))
      (unreachable))
    (throw_ref)))
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x18(%r11), %r11
;;       addq    $0x30, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x1d8
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movq    %r14, %rdi
;;       callq   0x3bb
;;       ├─╼ exception frame offset: SP = FP - 0x10
;;       ╰─╼ exception handler: tag=0, context at [SP+0x8], handler=0x129
;;       movq    8(%rsp), %r14
;;       movq    0x28(%r14), %rcx
;;       movl    0xc(%rcx), %ecx
;;       subq    $4, %rsp
;;       movl    %eax, (%rsp)
;;       subq    $4, %rsp
;;       movl    %ecx, (%rsp)
;;       subq    $8, %rsp
;;       movq    %r14, %rdi
;;       movl    $0x4000002, %esi
;;       movl    8(%rsp), %edx
;;       movl    $0x20, %ecx
;;       movl    $0x10, %r8d
;;       callq   0x2f4
;;       ├─╼ exception frame offset: SP = FP - 0x20
;;       ╰─╼ exception handler: tag=0, context at [SP+0x18], handler=0x129
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
;;       movl    %ecx, 0x10(%rdx)
;;       movl    $0, 0x14(%rdx)
;;       movl    $0, %ecx
;;       pushq   %rdx
;;       subq    $4, %rsp
;;       movl    %eax, (%rsp)
;;       pushq   %rcx
;;       subq    $0xc, %rsp
;;       movq    %r14, %rdi
;;       movq    0xc(%rsp), %rsi
;;       callq   0x343
;;       ├─╼ exception frame offset: SP = FP - 0x30
;;       ╰─╼ exception handler: tag=0, context at [SP+0x28], handler=0x129
;;       addq    $0xc, %rsp
;;       addq    $8, %rsp
;;       movq    0x14(%rsp), %r14
;;       movl    (%rsp), %ecx
;;       addq    $4, %rsp
;;       popq    %rdx
;;       movl    %eax, 0x18(%rdx)
;;       subq    $4, %rsp
;;       movl    %ecx, (%rsp)
;;       subq    $0xc, %rsp
;;       movq    %r14, %rdi
;;       movl    0xc(%rsp), %esi
;;       callq   0x3e8
;;       ├─╼ exception frame offset: SP = FP - 0x20
;;       ╰─╼ exception handler: tag=0, context at [SP+0x18], handler=0x129
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
;;       addq    $0x20, %r11
;;       cmpq    %rdx, %r11
;;       ja      0x1da
;;  157: movq    %rcx, %rdx
;;       addq    %rax, %rdx
;;       movl    0x18(%rdx), %ecx
;;       pushq   %rdx
;;       subq    $4, %rsp
;;       movl    %eax, (%rsp)
;;       subq    $4, %rsp
;;       movl    %ecx, (%rsp)
;;       movq    %r14, %rdi
;;       movl    (%rsp), %esi
;;       movl    $0xffffffff, %edx
;;       callq   0x370
;;       addq    $4, %rsp
;;       ╰─╼ stack_map: frame_size=32, frame_offsets=[4]
;;       movq    0x14(%rsp), %r14
;;       movl    (%rsp), %ecx
;;       addq    $4, %rsp
;;       popq    %rdx
;;       pushq   %rax
;;       movl    %ecx, %eax
;;       subq    $4, %rsp
;;       movl    %eax, (%rsp)
;;       subq    $4, %rsp
;;       movq    %r14, %rdi
;;       movl    4(%rsp), %esi
;;       callq   0x3e8
;;       addq    $4, %rsp
;;       ╰─╼ stack_map: frame_size=32, frame_offsets=[4]
;;       addq    $4, %rsp
;;       movq    0x10(%rsp), %r14
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;  1d8: ud2
;;  1da: ud2
