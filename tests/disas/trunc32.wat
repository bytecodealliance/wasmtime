;;! target = "x86_64-unknown-linux-gnu"
;;! test = "compile"
;;! flags = "-Ccompiler=cranelift -Ccranelift-has_sse41=false  -Osignals-based-traps=n"

(module
  (func (export "i32.trunc_f32_u") (param f32) (result i32)
    (i32.trunc_f32_u (local.get 0))
  )
)
;; wasm[0]::function[0]:
;;       movq    8(%rdi), %r10
;;       movq    0x10(%r10), %r10
;;       addq    $0x40, %r10
;;       cmpq    %rsp, %r10
;;       ja      0x2e8
;;   15: pushq   %rbp
;;       movq    %rsp, %rbp
;;       subq    $0x20, %rsp
;;       movq    %rbx, 0x10(%rsp)
;;       movq    %rdi, %rbx
;;       movdqu  %xmm0, (%rsp)
;;       xorpd   %xmm0, %xmm0
;;       movdqu  (%rsp), %xmm1
;;       cvtss2sd %xmm1, %xmm0
;;       ucomisd %xmm0, %xmm0
;;       jp      0xf7
;;       jne     0xf7
;;   47: movq    %rbx, %rdi
;;       callq   0x25d
;;       movabsq $13830554455654793216, %rax
;;       movq    %rax, %xmm1
;;       ucomisd %xmm0, %xmm1
;;       jae     0xe0
;;   68: ucomisd 0xb0(%rip), %xmm0
;;       jae     0xc9
;;   76: movdqu  (%rsp), %xmm2
;;       movl    $0x4f000000, %ecx
;;       movd    %ecx, %xmm1
;;       ucomiss %xmm1, %xmm2
;;       jae     0xa2
;;       jp      0x10e
;;   93: cvttss2si %xmm2, %eax
;;       cmpl    $0, %eax
;;       jge     0xbb
;;   a0: ud2
;;       movaps  %xmm2, %xmm0
;;       subss   %xmm1, %xmm0
;;       cvttss2si %xmm0, %eax
;;       cmpl    $0, %eax
;;       jl      0x110
;;   b6: addl    $0x80000000, %eax
;;       movq    0x10(%rsp), %rbx
;;       addq    $0x20, %rsp
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;   c9: movl    $6, %esi
;;   ce: movq    %rbx, %rdi
;;   d1: callq   0x28a
;;   d6: movq    %rbx, %rdi
;;   d9: callq   0x2bb
;;   de: ud2
;;   e0: movl    $6, %esi
;;   e5: movq    %rbx, %rdi
;;   e8: callq   0x28a
;;   ed: movq    %rbx, %rdi
;;   f0: callq   0x2bb
;;   f5: ud2
;;   f7: movl    $8, %esi
;;   fc: movq    %rbx, %rdi
;;   ff: callq   0x28a
;;  104: movq    %rbx, %rdi
;;  107: callq   0x2bb
;;  10c: ud2
;;  10e: ud2
;;  110: ud2
;;  112: addb    %al, (%rax)
;;  114: addb    %al, (%rax)
;;  116: addb    %al, (%rax)
;;  118: addb    %al, (%rax)
;;  11a: addb    %al, (%rax)
;;  11c: addb    %al, (%rax)
;;  11e: addb    %al, (%rax)
;;  120: addb    %al, (%rax)
;;  122: addb    %al, (%rax)
;;  124: addb    %al, (%rax)
;;  126: lock addb %al, (%r8)
;;  12a: addb    %al, (%rax)
;;  12c: addb    %al, (%rax)
;;  12e: addb    %al, (%rax)
