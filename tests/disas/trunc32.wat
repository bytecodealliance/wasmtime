;;! target = "x86_64-unknown-linux-gnu"
;;! test = "compile"
;;! flags = "-Ccompiler=cranelift -Ccranelift-has_sse41=false  -Osignals-based-traps=n"

(module
  (func (export "i32.trunc_f32_u") (param f32) (result i32)
    (i32.trunc_f32_u (local.get 0))
  )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       subq    $0x20, %rsp
;;       movq    %r12, 0x10(%rsp)
;;       movdqu  %xmm0, (%rsp)
;;       movq    8(%rdi), %rax
;;       movq    %rdi, %r12
;;       movq    0x10(%rax), %rax
;;       movq    %rsp, %rcx
;;       cmpq    %rax, %rcx
;;       jb      0x125
;;   29: xorpd   %xmm0, %xmm0
;;       movdqu  (%rsp), %xmm3
;;       cvtss2sd %xmm3, %xmm0
;;       ucomisd %xmm0, %xmm0
;;       jp      0x10e
;;       jne     0x10e
;;   46: movq    %r12, %rdi
;;       callq   0x21b
;;       movabsq $13830554455654793216, %r8
;;       movq    %r8, %xmm1
;;       ucomisd %xmm0, %xmm1
;;       jae     0xf7
;;   67: ucomisd 0x61(%rip), %xmm0
;;       jae     0xe0
;;   75: movdqu  (%rsp), %xmm7
;;       movl    $0x4f000000, %edi
;;       movd    %edi, %xmm2
;;       ucomiss %xmm2, %xmm7
;;       jae     0x99
;;       jp      0xc0
;;   8e: cvttss2si %xmm7, %eax
;;       cmpl    $0, %eax
;;       jge     0xb2
;;   97: ud2
;;       movaps  %xmm7, %xmm3
;;       subss   %xmm2, %xmm3
;;       cvttss2si %xmm3, %eax
;;       cmpl    $0, %eax
;;       jl      0xc2
;;   ad: addl    $0x80000000, %eax
;;       movq    0x10(%rsp), %r12
;;       addq    $0x20, %rsp
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;   c0: ud2
;;   c2: ud2
;;   c4: addb    %al, (%rax)
;;   c6: addb    %al, (%rax)
;;   c8: addb    %al, (%rax)
;;   ca: addb    %al, (%rax)
;;   cc: addb    %al, (%rax)
;;   ce: addb    %al, (%rax)
;;   d0: addb    %al, (%rax)
;;   d2: addb    %al, (%rax)
;;   d4: addb    %al, (%rax)
;;   d6: lock addb %al, (%r8)
;;   da: addb    %al, (%rax)
;;   dc: addb    %al, (%rax)
;;   de: addb    %al, (%rax)
;;   e0: movl    $6, %esi
;;   e5: movq    %r12, %rdi
;;   e8: callq   0x25a
;;   ed: movq    %r12, %rdi
;;   f0: callq   0x29d
;;   f5: ud2
;;   f7: movl    $6, %esi
;;   fc: movq    %r12, %rdi
;;   ff: callq   0x25a
;;  104: movq    %r12, %rdi
;;  107: callq   0x29d
;;  10c: ud2
;;  10e: movl    $8, %esi
;;  113: movq    %r12, %rdi
;;  116: callq   0x25a
;;  11b: movq    %r12, %rdi
;;  11e: callq   0x29d
;;  123: ud2
;;  125: xorl    %esi, %esi
;;  127: movq    %r12, %rdi
;;  12a: callq   0x25a
;;  12f: movq    %r12, %rdi
;;  132: callq   0x29d
;;  137: ud2
