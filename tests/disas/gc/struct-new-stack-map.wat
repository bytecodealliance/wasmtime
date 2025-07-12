;;! target = "x86_64"
;;! flags = "-W function-references,gc"
;;! test = "compile"

(module
  (type $ty (struct (field (mut f32))
                    (field (mut i8))
                    (field (mut anyref))))

  (func (param f32 i32 anyref) (result (ref $ty))
    (struct.new $ty (local.get 0) (local.get 1) (local.get 2))
  )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r10
;;       movq    0x10(%r10), %r10
;;       addq    $0x50, %r10
;;       cmpq    %rsp, %r10
;;       ja      0xc7
;;   19: subq    $0x40, %rsp
;;       movq    %r12, 0x20(%rsp)
;;       movq    %r13, 0x28(%rsp)
;;       movq    %r14, 0x30(%rsp)
;;       movq    %rdx, %r12
;;       movdqu  %xmm0, 8(%rsp)
;;       leaq    (%rsp), %r14
;;       movl    %ecx, (%r14)
;;       movl    $0xb0000000, %esi
;;       xorl    %edx, %edx
;;       movl    $0x28, %ecx
;;       movl    $8, %r8d
;;       movq    %rdi, %r13
;;       callq   0x15f
;;       movq    8(%r13), %r8
;;       ╰─╼ stack_map: frame_size=64, frame_offsets=[0]
;;       movq    0x18(%r8), %r8
;;       movq    %rax, %r10
;;       movl    %r10d, %r9d
;;       movdqu  8(%rsp), %xmm0
;;       movss   %xmm0, 0x18(%r8, %r9)
;;       movq    %r12, %rdx
;;       movb    %dl, 0x1c(%r8, %r9)
;;       movl    (%r14), %r11d
;;       movq    %r11, %rdx
;;       andl    $1, %edx
;;       testl   %r11d, %r11d
;;       sete    %sil
;;       movzbl  %sil, %esi
;;       orl     %esi, %edx
;;       testl   %edx, %edx
;;       jne     0xa4
;;   97: movl    %r11d, %ecx
;;       leaq    (%r8, %rcx), %rax
;;       addq    $1, 8(%r8, %rcx)
;;       movl    (%r14), %edx
;;       movl    %edx, 0x20(%r8, %r9)
;;       movq    %r10, %rax
;;       movq    0x20(%rsp), %r12
;;       movq    0x28(%rsp), %r13
;;       movq    0x30(%rsp), %r14
;;       addq    $0x40, %rsp
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;   c7: ud2
