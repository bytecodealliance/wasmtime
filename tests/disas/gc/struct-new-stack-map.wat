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
;;       addq    $0x40, %r10
;;       cmpq    %rsp, %r10
;;       ja      0xd3
;;   19: subq    $0x30, %rsp
;;       movq    %rbx, 0x20(%rsp)
;;       movq    %r12, 0x28(%rsp)
;;       movl    %ecx, (%rsp)
;;       movq    0x20(%rdi), %rsi
;;       movl    (%rsi), %eax
;;       movl    %eax, %ecx
;;       leaq    0x20(%rcx), %r8
;;       movl    4(%rsi), %r9d
;;       cmpq    %r9, %r8
;;       ja      0x94
;;   43: leal    0x20(%rax), %r8d
;;       movl    %r8d, (%rsi)
;;       movq    8(%rdi), %rsi
;;       movq    0x18(%rsi), %r8
;;       leaq    (%r8, %rcx), %rsi
;;       movl    $0xb0000022, (%r8, %rcx)
;;       movq    0x28(%rdi), %rdi
;;       movl    (%rdi), %edi
;;       movl    %edi, 4(%r8, %rcx)
;;       movl    $0x20, %edi
;;       movl    %edi, 8(%r8, %rcx)
;;       movss   %xmm0, 0x10(%rsi)
;;       movb    %dl, 0x14(%rsi)
;;       movl    (%rsp), %ecx
;;       movl    %ecx, 0x18(%rsi)
;;       movq    0x20(%rsp), %rbx
;;       movq    0x28(%rsp), %r12
;;       addq    $0x30, %rsp
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;   94: movq    %rdi, %rbx
;;   97: movq    %rdx, %r12
;;   9a: movdqu  %xmm0, 8(%rsp)
;;   a0: movl    $0xb0000022, %esi
;;   a5: movq    0x28(%rbx), %rax
;;   a9: movl    (%rax), %edx
;;   ab: movl    $0x20, %ecx
;;   b0: movl    $0x10, %r8d
;;   b6: callq   0x149
;;   bb: movq    8(%rbx), %rcx
;;       ╰─╼ stack_map: frame_size=48, frame_offsets=[0]
;;   bf: movl    %eax, %esi
;;   c1: addq    0x18(%rcx), %rsi
;;   c5: movq    %r12, %rdx
;;   c8: movdqu  8(%rsp), %xmm0
;;   ce: jmp     0x73
;;   d3: ud2
