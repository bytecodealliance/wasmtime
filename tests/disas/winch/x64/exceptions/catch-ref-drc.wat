;;! target = "x86_64"
;;! test = "winch"
;;! flags = "-W exceptions -C collector=drc"

;; A `catch_ref` landing pad preserves the exception reference while the DRC
;; read barrier loads an `externref` payload.
(module
  (tag $e (param externref))
  (func
    (block $h (result externref exnref)
      (try_table (catch_ref $e $h)
        (throw $e (ref.null extern)))
      (unreachable))
    (drop)
    (drop)))
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x18(%r11), %r11
;;       addq    $0x20, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x25d
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movq    %r14, %rdi
;;       callq   0x3c9
;;       ├─╼ exception frame offset: SP = FP - 0x10
;;       ╰─╼ exception handler: tag=0, context at [SP+0x8], handler=0x12c
;;       movq    8(%rsp), %r14
;;       movq    0x28(%r14), %rcx
;;       movl    0xc(%rcx), %ecx
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
;;       callq   0x37a
;;       ├─╼ exception frame offset: SP = FP - 0x20
;;       ╰─╼ exception handler: tag=0, context at [SP+0x18], handler=0x12c
;;       addq    $0xc, %rsp
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
;;       movl    $0, %ecx
;;       testl   %ecx, %ecx
;;       je      0x100
;;   b9: movl    %ecx, %r11d
;;       andl    $1, %r11d
;;       testl   %r11d, %r11d
;;       jne     0x100
;;   cc: movq    8(%r14), %rbx
;;       movq    0x28(%rbx), %rsi
;;       movq    0x20(%rbx), %rbx
;;       movq    %rcx, %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsi, %r11
;;       ja      0x25f
;;   eb: movq    %rbx, %rdi
;;       addq    %rcx, %rdi
;;       movq    8(%rdi), %r8
;;       addq    $1, %r8
;;       movq    %r8, 8(%rdi)
;;       movl    %ecx, 0x20(%rdx)
;;       subq    $4, %rsp
;;       movl    %eax, (%rsp)
;;       subq    $0xc, %rsp
;;       movq    %r14, %rdi
;;       movl    0xc(%rsp), %esi
;;       callq   0x3f6
;;       ├─╼ exception frame offset: SP = FP - 0x20
;;       ╰─╼ exception handler: tag=0, context at [SP+0x18], handler=0x12c
;;       addq    $0x10, %rsp
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
;;       ja      0x261
;;  15a: movq    %rcx, %rdx
;;       addq    %rax, %rdx
;;       movl    0x20(%rdx), %ecx
;;       pushq   %rdx
;;       subq    $4, %rsp
;;       movl    %eax, (%rsp)
;;       subq    $4, %rsp
;;       movl    %ecx, (%rsp)
;;       movl    (%rsp), %eax
;;       testl   %eax, %eax
;;       je      0x22c
;;  183: movl    %eax, %r11d
;;       andl    $1, %r11d
;;       testl   %r11d, %r11d
;;       jne     0x22c
;;  196: movq    8(%r14), %rcx
;;       movq    0x28(%rcx), %rdx
;;       movq    0x20(%rcx), %rcx
;;       movq    %rax, %r11
;;       addq    $0x14, %r11
;;       cmpq    %rdx, %r11
;;       ja      0x263
;;  1b5: movq    %rcx, %rdx
;;       addq    %rax, %rdx
;;       movl    (%rdx), %r11d
;;       andl    $2, %r11d
;;       testl   %r11d, %r11d
;;       jne     0x22c
;;  1ce: movq    0x20(%r14), %rbx
;;       movl    (%rbx), %r11d
;;       movl    %r11d, 0x10(%rdx)
;;       movl    (%rdx), %r11d
;;       orl     $2, %r11d
;;       movl    %r11d, (%rdx)
;;       movq    8(%rdx), %rsi
;;       addq    $1, %rsi
;;       movq    %rsi, 8(%rdx)
;;       movl    %eax, (%rbx)
;;       movl    4(%rbx), %esi
;;       addl    $1, %esi
;;       movl    %esi, 4(%rbx)
;;       movl    8(%rbx), %r11d
;;       addl    %r11d, %r11d
;;       cmpl    %r11d, %esi
;;       jb      0x22c
;;  213: cmpl    $0x400, %esi
;;       jb      0x22c
;;  21f: movq    %r14, %rdi
;;       callq   0x43d
;;       movq    0x18(%rsp), %r14
;;       ╰─╼ stack_map: frame_size=32, frame_offsets=[0, 4]
;;       movl    (%rsp), %eax
;;       addq    $4, %rsp
;;       movl    (%rsp), %ecx
;;       addq    $4, %rsp
;;       popq    %rdx
;;       subq    $4, %rsp
;;       movl    %eax, (%rsp)
;;       movl    %ecx, %eax
;;       addq    $4, %rsp
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;  25d: ud2
;;  25f: ud2
;;  261: ud2
;;  263: ud2
