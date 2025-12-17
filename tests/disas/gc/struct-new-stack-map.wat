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
;;       ja      0xb5
;;   19: subq    $0x30, %rsp
;;       movq    %r13, 0x20(%rsp)
;;       movq    %r14, 0x28(%rsp)
;;       movq    %rdx, %r14
;;       movdqu  %xmm0, 8(%rsp)
;;       movl    %ecx, (%rsp)
;;       movl    $0xb0000000, %esi
;;       xorl    %edx, %edx
;;       movl    $0x28, %ecx
;;       movl    $8, %r8d
;;       movq    %rdi, %r13
;;       callq   0x12f
;;       movq    8(%r13), %rdx
;;       ╰─╼ stack_map: frame_size=48, frame_offsets=[0]
;;       movq    0x20(%rdx), %rdx
;;       movl    %eax, %r8d
;;       movdqu  8(%rsp), %xmm0
;;       movss   %xmm0, 0x18(%rdx, %r8)
;;       movq    %r14, %r9
;;       movb    %r9b, 0x1c(%rdx, %r8)
;;       movl    (%rsp), %r9d
;;       movq    %r9, %rcx
;;       andl    $1, %ecx
;;       testl   %r9d, %r9d
;;       sete    %r10b
;;       movzbl  %r10b, %r10d
;;       orl     %r10d, %ecx
;;       testl   %ecx, %ecx
;;       jne     0x9a
;;   8d: movl    %r9d, %edi
;;       leaq    (%rdx, %rdi), %rcx
;;       addq    $1, 8(%rdx, %rdi)
;;       movl    (%rsp), %ecx
;;       movl    %ecx, 0x20(%rdx, %r8)
;;       movq    0x20(%rsp), %r13
;;       movq    0x28(%rsp), %r14
;;       addq    $0x30, %rsp
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;   b5: ud2
