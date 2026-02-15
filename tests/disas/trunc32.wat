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
;;       movq    %rbx, 0x10(%rsp)
;;       movdqu  %xmm0, (%rsp)
;;       movq    8(%rdi), %rax
;;       movq    %rdi, %rbx
;;       movq    0x18(%rax), %rax
;;       movq    %rsp, %rcx
;;       cmpq    %rax, %rcx
;;       jb      0x10d
;;   29: xorpd   %xmm0, %xmm0
;;       movdqu  (%rsp), %xmm1
;;       cvtss2sd %xmm1, %xmm0
;;       ucomisd %xmm0, %xmm0
;;       jp      0xf6
;;       jne     0xf6
;;   46: movq    %rbx, %rdi
;;       callq   0x242
;;       movabsq $13830554455654793216, %rax
;;       movq    %rax, %xmm1
;;       ucomisd %xmm0, %xmm1
;;       jae     0xdf
;;   67: ucomisd 0xc1(%rip), %xmm0
;;       jae     0xc8
;;   75: movdqu  (%rsp), %xmm2
;;       movl    $0x4f000000, %ecx
;;       movd    %ecx, %xmm1
;;       ucomiss %xmm1, %xmm2
;;       jae     0xa1
;;       jp      0x121
;;   92: cvttss2si %xmm2, %eax
;;       cmpl    $0, %eax
;;       jge     0xba
;;   9f: ud2
;;       movaps  %xmm2, %xmm0
;;       subss   %xmm1, %xmm0
;;       cvttss2si %xmm0, %eax
;;       cmpl    $0, %eax
;;       jl      0x123
;;   b5: addl    $0x80000000, %eax
;;       movq    0x10(%rsp), %rbx
;;       addq    $0x20, %rsp
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;   c8: movl    $6, %esi
;;   cd: movq    %rbx, %rdi
;;   d0: callq   0x26f
;;   d5: movq    %rbx, %rdi
;;   d8: callq   0x2a0
;;   dd: ud2
;;   df: movl    $6, %esi
;;   e4: movq    %rbx, %rdi
;;   e7: callq   0x26f
;;   ec: movq    %rbx, %rdi
;;   ef: callq   0x2a0
;;   f4: ud2
;;   f6: movl    $8, %esi
;;   fb: movq    %rbx, %rdi
;;   fe: callq   0x26f
;;  103: movq    %rbx, %rdi
;;  106: callq   0x2a0
;;  10b: ud2
;;  10d: xorl    %esi, %esi
;;  10f: movq    %rbx, %rdi
;;  112: callq   0x26f
;;  117: movq    %rbx, %rdi
;;  11a: callq   0x2a0
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
