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
;;       jb      0x10d
;;   29: xorpd   %xmm0, %xmm0
;;       movdqu  (%rsp), %xmm3
;;       cvtss2sd %xmm3, %xmm0
;;       ucomisd %xmm0, %xmm0
;;       jp      0xf6
;;       jne     0xf6
;;   46: movq    %r12, %rdi
;;       callq   0x222
;;       movabsq $13830554455654793216, %r8
;;       movq    %r8, %xmm1
;;       ucomisd %xmm0, %xmm1
;;       jae     0xdf
;;   67: ucomisd 0xc1(%rip), %xmm0
;;       jae     0xc8
;;   75: movdqu  (%rsp), %xmm7
;;       movl    $0x4f000000, %edi
;;       movd    %edi, %xmm2
;;       ucomiss %xmm2, %xmm7
;;       jae     0xa1
;;       jp      0x121
;;   92: cvttss2si %xmm7, %eax
;;       cmpl    $0, %eax
;;       jge     0xba
;;   9f: ud2
;;       movaps  %xmm7, %xmm3
;;       subss   %xmm2, %xmm3
;;       cvttss2si %xmm3, %eax
;;       cmpl    $0, %eax
;;       jl      0x123
;;   b5: addl    $0x80000000, %eax
;;       movq    0x10(%rsp), %r12
;;       addq    $0x20, %rsp
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;   c8: movl    $6, %esi
;;   cd: movq    %r12, %rdi
;;   d0: callq   0x261
;;   d5: movq    %r12, %rdi
;;   d8: callq   0x2a4
;;   dd: ud2
;;   df: movl    $6, %esi
;;   e4: movq    %r12, %rdi
;;   e7: callq   0x261
;;   ec: movq    %r12, %rdi
;;   ef: callq   0x2a4
;;   f4: ud2
;;   f6: movl    $8, %esi
;;   fb: movq    %r12, %rdi
;;   fe: callq   0x261
;;  103: movq    %r12, %rdi
;;  106: callq   0x2a4
;;  10b: ud2
;;  10d: xorl    %esi, %esi
;;  10f: movq    %r12, %rdi
;;  112: callq   0x261
;;  117: movq    %r12, %rdi
;;  11a: callq   0x2a4
;;  11f: ud2
;;  121: ud2
;;  123: ud2
;;  125: addb    %al, (%rax)
;;  127: addb    %al, (%rax)
;;  129: addb    %al, (%rax)
;;  12b: addb    %al, (%rax)
;;  12d: addb    %al, (%rax)
;;  12f: addb    %al, (%rax)
;;  131: addb    %al, (%rax)
;;  133: addb    %al, (%rax)
;;  135: addb    %dh, %al
;;  137: addb    %al, (%r8)
;;  13a: addb    %al, (%rax)
;;  13c: addb    %al, (%rax)
;;  13e: addb    %al, (%rax)
