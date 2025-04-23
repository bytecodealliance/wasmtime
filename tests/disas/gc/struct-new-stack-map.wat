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
;;       ja      0xcc
;;   19: subq    $0x40, %rsp
;;       movq    %r13, 0x20(%rsp)
;;       movq    %r14, 0x28(%rsp)
;;       movq    %r15, 0x30(%rsp)
;;       movq    %rdx, %r13
;;       movdqu  %xmm0, 8(%rsp)
;;       leaq    (%rsp), %r15
;;       movl    %ecx, (%r15)
;;       movl    $0xb0000000, %esi
;;       xorl    %edx, %edx
;;       movl    $0x20, %ecx
;;       movl    $8, %r8d
;;       movq    %rdi, %r14
;;       callq   0x165
;;       movq    8(%r14), %r9
;;       ╰─╼ stack_map: frame_size=64, frame_offsets=[0]
;;       movq    0x18(%r9), %r9
;;       movq    %rax, %r11
;;       movl    %r11d, %r10d
;;       movdqu  8(%rsp), %xmm0
;;       movss   %xmm0, 0x10(%r9, %r10)
;;       movq    %r13, %rdx
;;       movb    %dl, 0x14(%r9, %r10)
;;       movl    (%r15), %esi
;;       movq    %rsi, %r8
;;       andl    $1, %r8d
;;       testl   %esi, %esi
;;       sete    %dil
;;       movzbl  %dil, %edi
;;       orl     %edi, %r8d
;;       testl   %r8d, %r8d
;;       jne     0xa9
;;   99: movl    %esi, %ecx
;;       leaq    (%r9, %rcx), %rdx
;;       movl    $1, %edx
;;       addq    %rdx, 8(%r9, %rcx)
;;       movl    (%r15), %r8d
;;       movl    %r8d, 0x18(%r9, %r10)
;;       movq    %r11, %rax
;;       movq    0x20(%rsp), %r13
;;       movq    0x28(%rsp), %r14
;;       movq    0x30(%rsp), %r15
;;       addq    $0x40, %rsp
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;   cc: ud2
