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
;;       ja      0xcf
;;   19: subq    $0x40, %rsp
;;       movq    %r12, 0x20(%rsp)
;;       movq    %r13, 0x28(%rsp)
;;       movq    %r14, 0x30(%rsp)
;;       movq    %rdx, %r14
;;       movdqu  %xmm0, 8(%rsp)
;;       leaq    (%rsp), %r13
;;       movl    %ecx, (%r13)
;;       movl    $0xb0000000, %esi
;;       xorl    %edx, %edx
;;       movl    $0x20, %ecx
;;       movl    $8, %r8d
;;       movq    %rdi, %r12
;;       callq   0x168
;;       movq    8(%r12), %r8
;;       ╰─╼ stack_map: frame_size=64, frame_offsets=[0]
;;       movq    0x18(%r8), %r8
;;       movq    %rax, %r9
;;       movl    %r9d, %r10d
;;       movdqu  8(%rsp), %xmm0
;;       movss   %xmm0, 0x10(%r8, %r10)
;;       movq    %r14, %rdx
;;       movb    %dl, 0x14(%r8, %r10)
;;       movl    (%r13), %r11d
;;       movq    %r11, %rdx
;;       andl    $1, %edx
;;       testl   %r11d, %r11d
;;       sete    %sil
;;       movzbl  %sil, %esi
;;       orl     %esi, %edx
;;       testl   %edx, %edx
;;       jne     0xab
;;   9a: movl    %r11d, %ecx
;;       leaq    (%r8, %rcx), %rax
;;       movl    $1, %eax
;;       addq    %rax, 8(%r8, %rcx)
;;       movl    (%r13), %edx
;;       movl    %edx, 0x18(%r8, %r10)
;;       movq    %r9, %rax
;;       movq    0x20(%rsp), %r12
;;       movq    0x28(%rsp), %r13
;;       movq    0x30(%rsp), %r14
;;       addq    $0x40, %rsp
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;   cf: ud2
