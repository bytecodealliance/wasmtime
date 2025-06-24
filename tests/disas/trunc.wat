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
;;       jb      0x12c
;;   24: ucomisd %xmm0, %xmm0
;;       movdqu  %xmm0, (%rsp)
;;       setp    %r10b
;;       setne   %r11b
;;       orl     %r11d, %r10d
;;       testb   %r10b, %r10b
;;       jne     0x115
;;   41: movq    %r14, %rdi
;;       movdqu  (%rsp), %xmm0
;;       callq   0x244
;;       movabsq $13830554455654793216, %r11
;;       movq    %r11, %xmm4
;;       ucomisd %xmm0, %xmm4
;;       setae   %dil
;;       testb   %dil, %dil
;;       jne     0xfe
;;   6e: ucomisd 0xda(%rip), %xmm0
;;       setae   %dl
;;       testb   %dl, %dl
;;       jne     0xe7
;;   81: movdqu  (%rsp), %xmm2
;;       movabsq $0x43e0000000000000, %r9
;;       movq    %r9, %xmm7
;;       ucomisd %xmm7, %xmm2
;;       jae     0xb6
;;       jp      0x140
;;   a5: cvttsd2si %xmm2, %rax
;;       cmpq    $0, %rax
;;       jge     0xd9
;;   b4: ud2
;;       movaps  %xmm2, %xmm0
;;       subsd   %xmm7, %xmm0
;;       cvttsd2si %xmm0, %rax
;;       cmpq    $0, %rax
;;       jl      0x142
;;   cc: movabsq $9223372036854775808, %r9
;;       addq    %r9, %rax
;;       movq    0x10(%rsp), %r14
;;       addq    $0x20, %rsp
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;   e7: movl    $6, %esi
;;   ec: movq    %r14, %rdi
;;   ef: callq   0x283
;;   f4: movq    %r14, %rdi
;;   f7: callq   0x2c6
;;   fc: ud2
;;   fe: movl    $6, %esi
;;  103: movq    %r14, %rdi
;;  106: callq   0x283
;;  10b: movq    %r14, %rdi
;;  10e: callq   0x2c6
;;  113: ud2
;;  115: movl    $8, %esi
;;  11a: movq    %r14, %rdi
;;  11d: callq   0x283
;;  122: movq    %r14, %rdi
;;  125: callq   0x2c6
;;  12a: ud2
;;  12c: xorl    %esi, %esi
;;  12e: movq    %r14, %rdi
;;  131: callq   0x283
;;  136: movq    %r14, %rdi
;;  139: callq   0x2c6
;;  13e: ud2
;;  140: ud2
;;  142: ud2
;;  144: addb    %al, (%rax)
;;  146: addb    %al, (%rax)
;;  148: addb    %al, (%rax)
;;  14a: addb    %al, (%rax)
;;  14c: addb    %al, (%rax)
;;  14e: addb    %al, (%rax)
;;  150: addb    %al, (%rax)
;;  152: addb    %al, (%rax)
;;  154: addb    %al, (%rax)
;;  156: lock addb %al, (%r8)
;;  15a: addb    %al, (%rax)
;;  15c: addb    %al, (%rax)
;;  15e: addb    %al, (%rax)
