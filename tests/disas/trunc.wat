;;! target = "x86_64-unknown-linux-gnu"
;;! test = "compile"
;;! flags = "-Ccompiler=cranelift -Ccranelift-has_sse41=false  -Osignals-based-traps=n"

(module
  (func (export "i64.trunc_f64_u") (param f64) (result i64)
    (i64.trunc_f64_u (local.get 0))
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
;;   f0: callq   0x283
;;   f5: movq    %r14, %rdi
;;   f8: callq   0x2c6
;;   fd: ud2
;;   ff: movl    $6, %esi
;;  104: movq    %r14, %rdi
;;  107: callq   0x283
;;  10c: movq    %r14, %rdi
;;  10f: callq   0x2c6
;;  114: ud2
;;  116: movl    $8, %esi
;;  11b: movq    %r14, %rdi
;;  11e: callq   0x283
;;  123: movq    %r14, %rdi
;;  126: callq   0x2c6
;;  12b: ud2
;;  12d: xorl    %esi, %esi
;;  12f: movq    %r14, %rdi
;;  132: callq   0x283
;;  137: movq    %r14, %rdi
;;  13a: callq   0x2c6
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
