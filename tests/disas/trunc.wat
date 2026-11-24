;;! target = "x86_64-unknown-linux-gnu"
;;! test = "compile"
;;! flags = "-Ccompiler=cranelift -Ccranelift-has_sse41=false  -Osignals-based-traps=n"

(module
  (func (export "i64.trunc_f64_u") (param f64) (result i64)
    (i64.trunc_f64_u (local.get 0))
  )
)
;; wasm[0]::function[0]:
;;       movq    8(%rdi), %r10
;;       movq    0x10(%r10), %r10
;;       addq    $0x40, %r10
;;       cmpq    %rsp, %r10
;;       ja      0x2ea
;;   15: pushq   %rbp
;;       movq    %rsp, %rbp
;;       subq    $0x20, %rsp
;;       movq    %r13, 0x10(%rsp)
;;       movq    %rdi, %r13
;;       ucomisd %xmm0, %xmm0
;;       movdqu  %xmm0, (%rsp)
;;       jp      0x102
;;       jne     0x102
;;   3a: movq    %r13, %rdi
;;       movdqu  (%rsp), %xmm0
;;       callq   0x25f
;;       movabsq $13830554455654793216, %rax
;;       movq    %rax, %xmm1
;;       ucomisd %xmm0, %xmm1
;;       jae     0xeb
;;   60: ucomisd 0xb8(%rip), %xmm0
;;       jae     0xd4
;;   6e: movdqu  (%rsp), %xmm2
;;       movabsq $0x43e0000000000000, %rcx
;;       movq    %rcx, %xmm1
;;       ucomisd %xmm1, %xmm2
;;       jae     0xa3
;;       jp      0x119
;;   92: cvttsd2si %xmm2, %rax
;;       cmpq    $0, %rax
;;       jge     0xc6
;;   a1: ud2
;;       movaps  %xmm2, %xmm0
;;       subsd   %xmm1, %xmm0
;;       cvttsd2si %xmm0, %rax
;;       cmpq    $0, %rax
;;       jl      0x11b
;;   b9: movabsq $9223372036854775808, %rcx
;;       addq    %rcx, %rax
;;       movq    0x10(%rsp), %r13
;;       addq    $0x20, %rsp
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;   d4: movl    $6, %esi
;;   d9: movq    %r13, %rdi
;;   dc: callq   0x28c
;;   e1: movq    %r13, %rdi
;;   e4: callq   0x2bd
;;   e9: ud2
;;   eb: movl    $6, %esi
;;   f0: movq    %r13, %rdi
;;   f3: callq   0x28c
;;   f8: movq    %r13, %rdi
;;   fb: callq   0x2bd
;;  100: ud2
;;  102: movl    $8, %esi
;;  107: movq    %r13, %rdi
;;  10a: callq   0x28c
;;  10f: movq    %r13, %rdi
;;  112: callq   0x2bd
;;  117: ud2
;;  119: ud2
;;  11b: ud2
;;  11d: addb    %al, (%rax)
;;  11f: addb    %al, (%rax)
;;  121: addb    %al, (%rax)
;;  123: addb    %al, (%rax)
;;  125: addb    %dh, %al
;;  127: addb    %al, (%r8)
;;  12a: addb    %al, (%rax)
;;  12c: addb    %al, (%rax)
;;  12e: addb    %al, (%rax)
