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
;;       ja      0xd7
;;   19: subq    $0x30, %rsp
;;       movq    %rbx, 0x20(%rsp)
;;       movq    %r12, 0x28(%rsp)
;;       movq    %rdx, %r12
;;       movdqu  %xmm0, 8(%rsp)
;;       movl    %ecx, (%rsp)
;;       movq    0x20(%rdi), %rdx
;;       movl    (%rdx), %eax
;;       movl    %eax, %ecx
;;       leaq    0x20(%rcx), %rsi
;;       movl    4(%rdx), %r8d
;;       cmpq    %r8, %rsi
;;       ja      0xa4
;;   4c: movq    %rdi, %rbx
;;       leal    0x20(%rax), %esi
;;       movl    %esi, (%rdx)
;;       movq    8(%rbx), %rdx
;;       movq    0x20(%rdx), %rsi
;;       leaq    (%rsi, %rcx), %rdx
;;       movl    $0xb0000022, (%rsi, %rcx)
;;       movq    0x28(%rbx), %rdi
;;       movl    (%rdi), %edi
;;       movl    %edi, 4(%rsi, %rcx)
;;       movl    $0x20, %edi
;;       movl    %edi, 8(%rsi, %rcx)
;;       movdqu  8(%rsp), %xmm0
;;       movss   %xmm0, 0x10(%rdx)
;;       movq    %r12, %rcx
;;       movb    %cl, 0x14(%rdx)
;;       movl    (%rsp), %ecx
;;       movl    %ecx, 0x18(%rdx)
;;       movq    0x20(%rsp), %rbx
;;       movq    0x28(%rsp), %r12
;;       addq    $0x30, %rsp
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;   a4: movl    $0xb0000022, %esi
;;   a9: movq    0x28(%rdi), %rax
;;   ad: movq    %rdi, %rbx
;;   b0: movl    (%rax), %edx
;;   b2: movl    $0x20, %ecx
;;   b7: movl    $0x10, %r8d
;;   bd: callq   0x14d
;;   c2: movq    8(%rbx), %rcx
;;       ╰─╼ stack_map: frame_size=48, frame_offsets=[0]
;;   c6: movl    %eax, %edx
;;   c8: addq    0x20(%rcx), %rdx
;;   cc: movdqu  8(%rsp), %xmm0
;;   d2: jmp     0x80
;;   d7: ud2
