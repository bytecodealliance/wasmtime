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
;;       jbe     0x1a
;;       jmp     0x2e8
;;   1a: pushq   %rbp
;;       movq    %rsp, %rbp
;;       subq    $0x20, %rsp
;;       movq    %rbx, 0x10(%rsp)
;;       movq    %rdi, %rbx
;;       movdqu  %xmm0, (%rsp)
;;       xorpd   %xmm0, %xmm0
;;       movdqu  (%rsp), %xmm1
;;       cvtss2sd %xmm1, %xmm0
;;       ucomisd %xmm0, %xmm0
;;       jp      0xfc
;;       jne     0xfc
;;   4c: movq    %rbx, %rdi
;;       callq   0x25d
;;       movabsq $13830554455654793216, %rax
;;       movq    %rax, %xmm1
;;       ucomisd %xmm0, %xmm1
;;       jae     0xe5
;;   6d: ucomisd 0xab(%rip), %xmm0
;;       jae     0xce
;;   7b: movdqu  (%rsp), %xmm2
;;       movl    $0x4f000000, %ecx
;;       movd    %ecx, %xmm1
;;       ucomiss %xmm1, %xmm2
;;       jae     0xa7
;;       jp      0x113
;;   98: cvttss2si %xmm2, %eax
;;       cmpl    $0, %eax
;;       jge     0xc0
;;   a5: ud2
;;       movaps  %xmm2, %xmm0
;;       subss   %xmm1, %xmm0
;;       cvttss2si %xmm0, %eax
;;       cmpl    $0, %eax
;;       jl      0x115
;;   bb: addl    $0x80000000, %eax
;;       movq    0x10(%rsp), %rbx
;;       addq    $0x20, %rsp
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;   ce: movl    $6, %esi
;;   d3: movq    %rbx, %rdi
;;   d6: callq   0x28a
;;   db: movq    %rbx, %rdi
;;   de: callq   0x2bb
;;   e3: ud2
;;   e5: movl    $6, %esi
;;   ea: movq    %rbx, %rdi
;;   ed: callq   0x28a
;;   f2: movq    %rbx, %rdi
;;   f5: callq   0x2bb
;;   fa: ud2
;;   fc: movl    $8, %esi
;;  101: movq    %rbx, %rdi
;;  104: callq   0x28a
;;  109: movq    %rbx, %rdi
;;  10c: callq   0x2bb
;;  111: ud2
;;  113: ud2
;;  115: ud2
;;  117: addb    %al, (%rax)
;;  119: addb    %al, (%rax)
;;  11b: addb    %al, (%rax)
;;  11d: addb    %al, (%rax)
;;  11f: addb    %al, (%rax)
;;  121: addb    %al, (%rax)
;;  123: addb    %al, (%rax)
;;  125: addb    %dh, %al
;;  127: addb    %al, (%r8)
;;  12a: addb    %al, (%rax)
;;  12c: addb    %al, (%rax)
;;  12e: addb    %al, (%rax)
