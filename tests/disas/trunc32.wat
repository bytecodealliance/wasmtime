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
;;       movq    %r14, 0x10(%rsp)
;;       movq    8(%rdi), %rdx
;;       movq    %rdi, %r14
;;       movq    0x10(%rdx), %rdx
;;       movq    %rsp, %r8
;;       cmpq    %rdx, %r8
;;       jb      0x12d
;;   24: ucomisd %xmm0, %xmm0
;;       movdqu  %xmm0, (%rsp)
;;       setp    %r10b
;;       setne   %r11b
;;       orl     %r11d, %r10d
;;       testb   %r10b, %r10b
;;       jne     0x116
;;   41: movq    %r14, %rdi
;;       movdqu  (%rsp), %xmm0
;;       callq   0x244
;;       movabsq $13830554455654793216, %r11
;;       movq    %r11, %xmm4
;;       ucomisd %xmm0, %xmm4
;;       setae   %dil
;;       testb   %dil, %dil
;;       jne     0xff
;;   6e: ucomisd 0xda(%rip), %xmm0
;;       setae   %dl
;;       testb   %dl, %dl
;;       jne     0xe8
;;   82: movdqu  (%rsp), %xmm2
;;       movabsq $0x43e0000000000000, %r9
;;       movq    %r9, %xmm7
;;       ucomisd %xmm7, %xmm2
;;       jae     0xb7
;;       jp      0x141
;;   a6: cvttsd2si %xmm2, %rax
;;       cmpq    $0, %rax
;;       jge     0xda
;;   b5: ud2
;;       movaps  %xmm2, %xmm0
;;       subsd   %xmm7, %xmm0
;;       cvttsd2si %xmm0, %rax
;;       cmpq    $0, %rax
;;       jl      0x143
;;   cd: movabsq $9223372036854775808, %r9
;;       addq    %r9, %rax
;;       movq    0x10(%rsp), %r14
;;       addq    $0x20, %rsp
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;   e8: movl    $6, %esi
;;   ed: movq    %r14, %rdi
;;   f0: callq   0x284
;;   f5: movq    %r14, %rdi
;;   f8: callq   0x2c8
;;   fd: ud2
;;   ff: movl    $6, %esi
;;  104: movq    %r14, %rdi
;;  107: callq   0x284
;;  10c: movq    %r14, %rdi
;;  10f: callq   0x2c8
;;  114: ud2
;;  116: movl    $8, %esi
;;  11b: movq    %r14, %rdi
;;  11e: callq   0x284
;;  123: movq    %r14, %rdi
;;  126: callq   0x2c8
;;  12b: ud2
;;  12d: xorl    %esi, %esi
;;  12f: movq    %r14, %rdi
;;  132: callq   0x284
;;  137: movq    %r14, %rdi
;;  13a: callq   0x2c8
;;  13f: ud2
;;  141: ud2
;;  143: ud2
;;  145: addb    %al, (%rax)
;;  147: addb    %al, (%rax)
;;  149: addb    %al, (%rax)
;;  14b: addb    %al, (%rax)
;;  14d: addb    %al, (%rax)
;;  14f: addb    %al, (%rax)
;;  151: addb    %al, (%rax)
;;  153: addb    %al, (%rax)
;;  155: addb    %dh, %al
;;  157: addb    %al, (%r8)
;;  15a: addb    %al, (%rax)
;;  15c: addb    %al, (%rax)
;;  15e: addb    %al, (%rax)

;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       subq    $0x20, %rsp
;;       movq    %r12, 0x10(%rsp)
;;       movdqu  %xmm0, (%rsp)
;;       movq    8(%rdi), %r10
;;       movq    %rdi, %r12
;;       movq    0x10(%r10), %r10
;;       movq    %rsp, %r11
;;       cmpq    %r10, %r11
;;       jb      0x122
;;   29: xorpd   %xmm0, %xmm0
;;       movdqu  (%rsp), %xmm4
;;       cvtss2sd %xmm4, %xmm0
;;       ucomisd %xmm0, %xmm0
;;       setp    %dil
;;       setne   %al
;;       orl     %eax, %edi
;;       testb   %dil, %dil
;;       jne     0x10b
;;   4d: movq    %r12, %rdi
;;       callq   0x232
;;       movabsq $13830554455654793216, %rax
;;       movq    %rax, %xmm7
;;       ucomisd %xmm0, %xmm7
;;       setae   %dl
;;       testb   %dl, %dl
;;       jne     0xf4
;;   74: ucomisd 0xc4(%rip), %xmm0
;;       setae   %r10b
;;       testb   %r10b, %r10b
;;       jne     0xdd
;;   89: movdqu  (%rsp), %xmm4
;;       movl    $0x4f000000, %esi
;;       movd    %esi, %xmm2
;;       ucomiss %xmm2, %xmm4
;;       jae     0xb5
;;       jp      0x136
;;   a6: cvttss2si %xmm4, %eax
;;       cmpl    $0, %eax
;;       jge     0xcf
;;   b3: ud2
;;       movaps  %xmm4, %xmm3
;;       subss   %xmm2, %xmm3
;;       cvttss2si %xmm3, %eax
;;       cmpl    $0, %eax
;;       jl      0x138
;;   c9: addl    $0x80000000, %eax
;;       movq    0x10(%rsp), %r12
;;       addq    $0x20, %rsp
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;   dd: movl    $6, %esi
;;   e2: movq    %r12, %rdi
;;   e5: callq   0x272
;;   ea: movq    %r12, %rdi
;;   ed: callq   0x2b6
;;   f2: ud2
;;   f4: movl    $6, %esi
;;   f9: movq    %r12, %rdi
;;   fc: callq   0x272
;;  101: movq    %r12, %rdi
;;  104: callq   0x2b6
;;  109: ud2
;;  10b: movl    $8, %esi
;;  110: movq    %r12, %rdi
;;  113: callq   0x272
;;  118: movq    %r12, %rdi
;;  11b: callq   0x2b6
;;  120: ud2
;;  122: xorl    %esi, %esi
;;  124: movq    %r12, %rdi
;;  127: callq   0x272
;;  12c: movq    %r12, %rdi
;;  12f: callq   0x2b6
;;  134: ud2
;;  136: ud2
;;  138: ud2
;;  13a: addb    %al, (%rax)
;;  13c: addb    %al, (%rax)
;;  13e: addb    %al, (%rax)
;;  140: addb    %al, (%rax)
;;  142: addb    %al, (%rax)
;;  144: addb    %al, (%rax)
;;  146: lock addb %al, (%r8)
;;  14a: addb    %al, (%rax)
;;  14c: addb    %al, (%rax)
;;  14e: addb    %al, (%rax)
