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
;;       subq    $0x40, %rsp
;;       movq    %r13, 0x20(%rsp)
;;       movq    %r14, 0x28(%rsp)
;;       movq    %r15, 0x30(%rsp)
;;       movq    %rdx, %r15
;;       movdqu  %xmm0, 8(%rsp)
;;       leaq    (%rsp), %r14
;;       movl    %ecx, (%r14)
;;       movl    $0xb0000000, %esi
;;       xorl    %edx, %edx
;;       movl    $0x20, %ecx
;;       movl    $8, %r8d
;;       movq    %rdi, %r13
;;       callq   0x18a
;;       movq    0x28(%r13), %r9
;;       ╰─╼ stack_map: frame_size=64, frame_offsets=[0]
;;       movq    %rax, %r8
;;       movl    %r8d, %r10d
;;       movdqu  8(%rsp), %xmm0
;;       movss   %xmm0, 0x10(%r9, %r10)
;;       movq    %r15, %rdx
;;       movb    %dl, 0x14(%r9, %r10)
;;       movl    (%r14), %r11d
;;       movq    %r11, %rdx
;;       andl    $1, %edx
;;       testl   %r11d, %r11d
;;       sete    %sil
;;       movzbl  %sil, %esi
;;       orl     %esi, %edx
;;       testl   %edx, %edx
;;       jne     0xac
;;   7e: movl    %r11d, %edi
;;       addq    $8, %rdi
;;       jb      0xcf
;;   8b: movq    %rdi, %rcx
;;       addq    $8, %rcx
;;       jb      0xd1
;;   98: cmpq    0x30(%r13), %rcx
;;       ja      0xd3
;;   a2: movl    $1, %r11d
;;       addq    %r11, (%r9, %rdi)
;;       movl    (%r14), %r11d
;;       movl    %r11d, 0x18(%r9, %r10)
;;       movq    %r8, %rax
;;       movq    0x20(%rsp), %r13
;;       movq    0x28(%rsp), %r14
;;       movq    0x30(%rsp), %r15
;;       addq    $0x40, %rsp
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;   cf: ud2
;;   d1: ud2
;;   d3: ud2
