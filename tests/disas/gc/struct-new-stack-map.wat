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
;;       movq    0x18(%r10), %r10
;;       addq    $0x40, %r10
;;       cmpq    %rsp, %r10
;;       ja      0xaf
;;   19: subq    $0x30, %rsp
;;       movq    %rbx, 0x20(%rsp)
;;       movq    %r12, 0x28(%rsp)
;;       movq    %rdx, %r12
;;       movdqu  %xmm0, 8(%rsp)
;;       movl    %ecx, (%rsp)
;;       movl    $0xb0000000, %esi
;;       xorl    %edx, %edx
;;       movl    $0x28, %ecx
;;       movl    $8, %r8d
;;       movq    %rdi, %rbx
;;       callq   0x125
;;       movq    8(%rbx), %rcx
;;       ╰─╼ stack_map: frame_size=48, frame_offsets=[0]
;;       movq    0x20(%rcx), %rcx
;;       movl    %eax, %edx
;;       movdqu  8(%rsp), %xmm0
;;       movss   %xmm0, 0x18(%rcx, %rdx)
;;       movq    %r12, %rsi
;;       movb    %sil, 0x1c(%rcx, %rdx)
;;       movl    (%rsp), %esi
;;       movq    %rsi, %rdi
;;       andl    $1, %edi
;;       testl   %esi, %esi
;;       sete    %r8b
;;       movzbl  %r8b, %r8d
;;       orl     %r8d, %edi
;;       testl   %edi, %edi
;;       jne     0x95
;;   89: movl    %esi, %esi
;;       leaq    (%rcx, %rsi), %rdi
;;       addq    $1, 8(%rcx, %rsi)
;;       movl    (%rsp), %esi
;;       movl    %esi, 0x20(%rcx, %rdx)
;;       movq    0x20(%rsp), %rbx
;;       movq    0x28(%rsp), %r12
;;       addq    $0x30, %rsp
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;   af: ud2
